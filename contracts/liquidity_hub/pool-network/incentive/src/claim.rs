use std::collections::HashMap;

use cosmwasm_std::{
    to_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal256, Deps, DepsMut, Env, MessageInfo, Order,
    StdError, StdResult, Uint128, Uint256, WasmMsg,
};

use white_whale::pool_network::{
    asset::AssetInfo,
    incentive::{Curve, Flow},
};

use crate::state::{ADDRESS_WEIGHT_HISTORY, CONFIG, GLOBAL_WEIGHT_SNAPSHOT, LAST_CLAIMED_EPOCH};
use crate::{
    error::ContractError,
    state::{ADDRESS_WEIGHT, FLOWS, GLOBAL_WEIGHT, LAST_CLAIMED_INDEX},
};

// for flow in index.flows:
//      emissions = calculate_emission(current_time, max(last_claimed, flow.config.start), flow.config.end, flow.curve, flow.asset.total)
//      user_rewards = share * (emissions - flow.asset.claimed)
//      send user_rewards to sender
//      flow.asset.claimed += user_rewards
// todo remove
pub fn calculate_emission(current_time: u64, start: u64, end: u64, flow: &Flow) -> Uint256 {
    /* // print all inputs
    println!("current_time: {}", current_time);
    println!("start: {}", start);
    println!("end: {}", end);
    println!("flow: {:?}", flow);

    let total = Uint256::from_uint128(flow.flow_asset.amount);
    println!("total: {}", total);

    let percentage = Decimal256::from_ratio(current_time - start, end - start);
    println!("current_time - start: {}", current_time - start);
    println!("end - start: {}", end - start);
    println!("percentage: {}", percentage);
    match &flow.curve {
        &Curve::Linear => percentage * total,
    }*/

    // if the flow has not started yet, return 0
    if flow.start_timestamp > current_time {
        return Uint256::zero();
    }

    // V3

    // print all inputs
    // print all inputs
    println!("current_time: {}", current_time);
    println!("start: {}", start);
    println!("end: {}", end);
    println!("flow: {:?}", flow);

    let total = Uint256::from_uint128(flow.flow_asset.amount);
    println!("total: {}", total);

    let claimed = Uint256::from_uint128(flow.claimed_amount);
    println!("claimed: {}", claimed);

    let remaining = total - claimed;
    println!("remaining: {}", remaining);

    if current_time >= end {
        return remaining;
    }

    let elapsed_percentage = Decimal256::from_ratio(current_time - start, end - start);
    println!("current_time - start: {}", current_time - start);
    println!("end - start: {}", end - start);
    println!("elapsed_percentage: {}", elapsed_percentage);

    match &flow.curve {
        &Curve::Linear => elapsed_percentage * remaining,
    }
}
// todo remove
pub fn get_user_share(deps: &Deps, address: Addr) -> Result<Decimal256, StdError> {
    // calculate user share
    let user_weight = ADDRESS_WEIGHT
        .may_load(deps.storage, address)?
        .unwrap_or_default();
    let global_weight = GLOBAL_WEIGHT.load(deps.storage)?;

    println!("user_weight: {}", user_weight);
    println!("global_weight: {}", global_weight);
    let user_share = Decimal256::from_ratio(user_weight, global_weight);
    Ok(user_share)
}
// todo remove
pub fn calculate_claimable_amount(
    flow: &Flow,
    env: &Env,
    user_last_claimed: u64,
    user_share: Decimal256,
) -> Result<Uint128, StdError> {
    let emissions = calculate_emission(
        env.block.time.seconds().min(flow.end_timestamp),
        user_last_claimed.max(flow.start_timestamp),
        flow.end_timestamp,
        &flow,
    );

    println!("emissions: {}", emissions);

    // convert back into Uint128
    Ok((user_share * emissions).try_into()?)
}

//todo remove
/// Performs the claim function, returning all the [`CosmosMsg`]'s to run.
pub fn claim(
    deps: &mut DepsMut,
    env: &Env,
    info: &MessageInfo,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let user_share = get_user_share(&deps.as_ref(), info.sender.clone())?;

    // calculate flow rewards
    let mut messages: Vec<CosmosMsg> = vec![];
    // let mut flows: Vec<Flow> = FLOWS
    //     .may_load(deps.storage)?
    //     .unwrap_or_default()
    //     .into_iter()
    //     .filter(|flow| flow.start_timestamp <= env.block.time.seconds())
    //     .collect();

    let mut flows = vec![];

    let user_last_claimed = LAST_CLAIMED_INDEX
        .may_load(deps.storage, info.sender.clone())?
        // .unwrap_or(env.block.time.seconds());
        .unwrap_or(0u64);

    for mut flow in flows.iter_mut() {
        let user_reward = calculate_claimable_amount(flow, env, user_last_claimed, user_share)?;

        if user_reward.is_zero() {
            // we don't want to construct a transfer message for them
            continue;
        }

        flow.claimed_amount = flow.claimed_amount.checked_add(user_reward)?;

        match &flow.flow_asset.info {
            AssetInfo::NativeToken { denom } => messages.push(
                BankMsg::Send {
                    to_address: info.sender.clone().into_string(),
                    amount: vec![Coin {
                        amount: user_reward,
                        denom: denom.to_owned(),
                    }],
                }
                .into(),
            ),
            AssetInfo::Token { contract_addr } => messages.push(
                WasmMsg::Execute {
                    contract_addr: contract_addr.to_owned(),
                    msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                        recipient: info.sender.clone().into_string(),
                        amount: user_reward,
                    })?,
                    funds: vec![],
                }
                .into(),
            ),
        }
    }

    // save the new flow state
    //FLOWS.save(deps.storage, &flows)?;

    LAST_CLAIMED_INDEX.save(deps.storage, info.sender.clone(), &env.block.time.seconds())?;

    Ok(messages)
}

// TODO new stuff, remove/refactor old stuff

/// Performs the claim function, returning all the [`CosmosMsg`]'s to run.
pub fn claim2(
    deps: &mut DepsMut,
    env: &Env,
    info: &MessageInfo,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let epoch_response: white_whale::fee_distributor::EpochResponse =
        deps.querier.query_wasm_smart(
            config.fee_distributor_address.into_string(),
            &white_whale::fee_distributor::QueryMsg::CurrentEpoch {},
        )?;

    let current_epoch = epoch_response.epoch.id.u64();
    let last_claimed_epoch = LAST_CLAIMED_EPOCH.may_load(deps.storage, &info.sender.clone())?;

    if let Some(last_claimed_epoch) = last_claimed_epoch {
        // if the last claimed epoch is the same as the current epoch, then there is nothing to claim
        if current_epoch == last_claimed_epoch {
            return Err(ContractError::NothingToClaim {});
        }
    }

    // let user_share = get_user_share(&deps.as_ref(), info.sender.clone())?;

    // calculate flow rewards
    let mut messages: Vec<CosmosMsg> = vec![];
    // let mut flows: Vec<Flow> = FLOWS
    //     .may_load(deps.storage)?
    //     .unwrap_or_default()
    //     .into_iter()
    //     //.filter(|flow| flow.start_timestamp <= env.block.time.seconds())
    //     .filter(|flow| flow.start_epoch <= current_epoch)
    //     .collect();
    let mut flows: Vec<Flow> = FLOWS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<((u64, u64), Flow)>>>()?
        .into_iter()
        .filter(|((start_epoch, _), _)| start_epoch <= &current_epoch)
        .map(|((_, _), flow)| flow)
        .collect::<Vec<Flow>>();

    println!("current_epoch: {:?}", current_epoch);
    println!("flows: {:?}", flows);

    //
    // let user_last_claimed = LAST_CLAIMED_INDEX
    //     .may_load(deps.storage, info.sender.clone())?
    //     // .unwrap_or(env.block.time.seconds());
    //     .unwrap_or(0u64);

    let mut last_user_weight_seen: (u64, Uint128) = (064, Uint128::zero());

    for mut flow in flows.iter_mut() {
        println!("flow: {:?}", flow);

        // check if flow already ended and if everything has been claimed for that flow.
        if current_epoch > flow.end_epoch && flow.claimed_amount == flow.flow_asset.amount {
            // if so, skip flow.
            continue;
        }

        last_user_weight_seen = (064, Uint128::zero());
        // check what is the earliest available weight for the user
        let first_available_weight_for_user = ADDRESS_WEIGHT_HISTORY
            .prefix(&info.sender.clone())
            .range(deps.storage, None, None, Order::Ascending)
            .take(1)
            .map(|item| Ok(item?))
            .collect::<StdResult<Vec<(u64, Uint128)>>>()?;

        println!(
            "first_available_weight_for_user: {:?}",
            first_available_weight_for_user
        );

        if !first_available_weight_for_user.is_empty() {
            last_user_weight_seen = first_available_weight_for_user[0];
        }

        println!("last_user_weight_seen: {:?}", last_user_weight_seen);

        let first_claimable_epoch = if let Some(last_claimed_epoch) = last_claimed_epoch {
            // start claiming from the last claimed epoch + 1
            last_claimed_epoch + 1
        } else {
            // the user never claimed before
            if flow.start_epoch > last_user_weight_seen.0 {
                // it means the user locked tokens before the flow started. Start from there just to get
                // the ADDRESS_WEIGHT_HISTORY right
                last_user_weight_seen.0
            } else {
                // it means the user locked tokens after the flow started, and last_user_weight_seen.0 has a value
                flow.start_epoch
            }
        };

        println!("first_claimable_epoch: {:?}", first_claimable_epoch);

        println!(
            "*** flow_emitted_tokens ***: {:?}",
            flow.emitted_tokens.clone()
        );
        let mut total_reward = Uint128::zero();

        for epoch_id in first_claimable_epoch..=current_epoch {
            println!("-------");
            println!("epoch_id: {:?}, flow_id: {}", epoch_id, flow.flow_id);
            // check if the flow is active in this epoch
            if epoch_id < flow.start_epoch {
                println!("this flow has not started yet: {}", flow.flow_id);

                // the flow is not active yet, skip
                continue;
            } else if epoch_id >= flow.end_epoch {
                // this flow has finished
                println!("this flow has already finished: {}", flow.flow_id);
                break;
            }

            // calculate emissions per epoch
            let emitted_tokens = if flow.emitted_tokens.is_empty() {
                Uint128::zero()
            } else {
                let previous_emission = flow
                    .emitted_tokens
                    .get(&(epoch_id - 1))
                    .unwrap_or(&Uint128::zero())
                    .clone();
                println!("previous_emission: {:?}", previous_emission);

                previous_emission

                // flow.emitted_tokens
                //     .get(&(epoch_id - 1))
                //     .unwrap_or(&Uint128::zero())
                //     .clone()

                // flow.emitted_tokens
                //     .get(&epoch_id)
                //     .unwrap_or(&previous_emission)
                //     .clone()
            };

            println!("emitted_tokens: {:?}", emitted_tokens);
            println!("flow.end_epoch: {:?}", flow.end_epoch);

            // emission = (total_tokens - emitted_tokens_at_epoch) / (flow_start + flow_duration - epoch) = (total_tokens - emitted_tokens_at_epoch) / (flow_end - epoch)
            let emission_per_epoch = flow
                .flow_asset
                .amount
                .saturating_sub(emitted_tokens)
                .checked_div(Uint128::from(flow.end_epoch - epoch_id))?;

            if flow.emitted_tokens.get(&epoch_id).is_none() {
                flow.emitted_tokens
                    .insert(epoch_id, emission_per_epoch.checked_add(emitted_tokens)?);
            }
            println!("emission_per_epoch: {:?}", emission_per_epoch);

            // get user weight
            let user_weight_at_epoch =
                ADDRESS_WEIGHT_HISTORY.may_load(deps.storage, (&info.sender.clone(), epoch_id))?;

            let user_weight = if let Some(user_weight_at_epoch) = user_weight_at_epoch {
                last_user_weight_seen = (epoch_id, user_weight_at_epoch);
                user_weight_at_epoch
            } else if last_user_weight_seen.0 != 0u64 && last_user_weight_seen.0 <= epoch_id {
                // we have a weight for this user for a previous epoch, but not this one, so we use
                // the last we saw as that's the current one
                last_user_weight_seen.1
            } else {
                // we don't have a weight for this user for this epoch, or any previous epoch, so we
                // skip this epoch. i.e. the user's weight is 0 for this epoch
                continue;
            };

            println!("user_weight: {:?}", user_weight);

            let global_weight_at_epoch = GLOBAL_WEIGHT_SNAPSHOT
                .may_load(deps.storage, epoch_id)?
                .unwrap_or_default();

            println!("global_weight_at_epoch: {:?}", global_weight_at_epoch);

            if global_weight_at_epoch == Uint128::zero() {
                // Nothing to compute here as the global weight is 0, we don't want to divide by 0
                continue;
            }

            let user_share_at_epoch = Decimal256::from_ratio(user_weight, global_weight_at_epoch);

            println!("user_share_at_epoch: {:?}", user_share_at_epoch);

            // // calculate emissions per epoch
            // let emitted_tokens = if flow.emitted_tokens.is_empty() {
            //     Uint128::zero()
            // } else {
            //     let previous_emission = flow
            //         .emitted_tokens
            //         .get(&(epoch_id - 1))
            //         .unwrap_or(&Uint128::zero())
            //         .clone();
            //     println!("previous_emission: {:?}", previous_emission);
            //
            //     flow.emitted_tokens
            //         .get(&epoch_id)
            //         .unwrap_or(&previous_emission)
            //         .clone()
            // };
            //
            //
            // println!("emitted_tokens: {:?}", emitted_tokens);
            // println!("flow.end_epoch: {:?}", flow.end_epoch);
            //
            // // emission = (total_tokens - emitted_tokens_at_epoch) / (flow_start + flow_duration - epoch) = (total_tokens - emitted_tokens_at_epoch) / (flow_end - epoch)
            // let emission_per_epoch = flow
            //     .flow_asset
            //     .amount
            //     .saturating_sub(emitted_tokens)
            //     .checked_div(Uint128::from(flow.end_epoch - epoch_id))?;
            //
            // if flow.emitted_tokens.get(&epoch_id).is_none() {
            //     flow.emitted_tokens.insert(epoch_id, emission_per_epoch.checked_add(emitted_tokens)?);
            // }
            //
            // println!("emission_per_epoch: {:?}", emission_per_epoch);

            // calculate user reward

            let user_reward = Uint256::from_uint128(emission_per_epoch) * user_share_at_epoch;

            let user_reward_at_epoch: Uint128 = user_reward.try_into()?;

            println!("user_reward_at_epoch: {:?}", user_reward_at_epoch);

            // sanity check for user_reward_at_epoch
            if user_reward_at_epoch > emission_per_epoch
                || user_reward_at_epoch.checked_add(flow.claimed_amount)? > flow.flow_asset.amount
            {
                return Err(ContractError::InvalidReward {});
            }

            if user_reward_at_epoch.is_zero() {
                // we don't want to construct a transfer message for the user
                continue;
            }

            flow.claimed_amount = flow.claimed_amount.checked_add(user_reward_at_epoch)?;

            match &flow.flow_asset.info {
                AssetInfo::NativeToken { denom } => messages.push(
                    BankMsg::Send {
                        to_address: info.sender.clone().into_string(),
                        amount: vec![Coin {
                            amount: user_reward_at_epoch,
                            denom: denom.to_owned(),
                        }],
                    }
                    .into(),
                ),
                AssetInfo::Token { contract_addr } => messages.push(
                    WasmMsg::Execute {
                        contract_addr: contract_addr.to_owned(),
                        msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                            recipient: info.sender.clone().into_string(),
                            amount: user_reward_at_epoch,
                        })?,
                        funds: vec![],
                    }
                    .into(),
                ),
            }

            total_reward += user_reward_at_epoch;
        }

        // save current flow state
        FLOWS.save(deps.storage, (flow.start_epoch, flow.flow_id), &flow)?;
        //FLOWS.save(deps.storage, &flow_id, &flow)?;

        println!("total_reward: {:?}", total_reward);

        //let user_reward = calculate_claimable_amount(flow, env, user_last_claimed, user_share)?;

        // if user_reward.is_zero() {
        //     // we don't want to construct a transfer message for them
        //     continue;
        // }
        //
        // flow.claimed_amount = flow.claimed_amount.checked_add(user_reward)?;
        //
        // match &flow.flow_asset.info {
        //     AssetInfo::NativeToken { denom } => messages.push(
        //         BankMsg::Send {
        //             to_address: info.sender.clone().into_string(),
        //             amount: vec![Coin {
        //                 amount: user_reward,
        //                 denom: denom.to_owned(),
        //             }],
        //         }
        //         .into(),
        //     ),
        //     AssetInfo::Token { contract_addr } => messages.push(
        //         WasmMsg::Execute {
        //             contract_addr: contract_addr.to_owned(),
        //             msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
        //                 recipient: info.sender.clone().into_string(),
        //                 amount: user_reward,
        //             })?,
        //             funds: vec![],
        //         }
        //         .into(),
        //     ),
        // }
    }

    println!("after claiming loop");

    // update the last seen weight for the user
    //delete all the entries in ADDRESS_WEIGHT_HISTORY that has prefix  info.sender.clone()
    let epoch_keys = ADDRESS_WEIGHT_HISTORY
        .prefix(&info.sender.clone())
        .keys(deps.storage, None, None, Order::Ascending)
        .map(|k| {
            let key = k?;
            println!("key is {:?}", key);
            Ok(key)
        })
        .collect::<StdResult<Vec<u64>>>()?;

    epoch_keys.iter().for_each(|&k| {
        println!("deleting key {:?}", (&info.sender.clone(), k.clone()));
        ADDRESS_WEIGHT_HISTORY.remove(deps.storage, (&info.sender.clone(), k.clone()));
    });

    ADDRESS_WEIGHT_HISTORY.update::<_, StdError>(
        deps.storage,
        (&info.sender.clone(), current_epoch + 1),
        |_| Ok(last_user_weight_seen.1),
    )?;

    println!(
        "last_user_weight_seen by {} is {:?} in epoch {}",
        info.sender.clone(),
        last_user_weight_seen.1,
        current_epoch + 1
    );

    LAST_CLAIMED_EPOCH.save(deps.storage, &info.sender.clone(), &current_epoch)?;

    println!(
        "last_claimed_epoch by {} is {}",
        info.sender.clone(),
        current_epoch
    );
    // save the new flow state
    //FLOWS.save(deps.storage, &flows)?;

    //todo remove
    LAST_CLAIMED_INDEX.save(deps.storage, info.sender.clone(), &env.block.time.seconds())?;
    println!("////////");
    Ok(messages)
}
