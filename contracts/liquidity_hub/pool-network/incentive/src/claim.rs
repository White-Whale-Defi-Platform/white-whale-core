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

/// Performs the claim function, returning all the [`CosmosMsg`]'s to run.
pub fn claim(
    deps: &mut DepsMut,
    env: &Env,
    info: &MessageInfo,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let user_share = get_user_share(&deps.as_ref(), info.sender.clone())?;

    // calculate flow rewards
    let mut messages: Vec<CosmosMsg> = vec![];
    let mut flows: Vec<Flow> = FLOWS
        .may_load(deps.storage)?
        .unwrap_or_default()
        .into_iter()
        .filter(|flow| flow.start_timestamp <= env.block.time.seconds())
        .collect();

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
    FLOWS.save(deps.storage, &flows)?;

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

    // let user_share = get_user_share(&deps.as_ref(), info.sender.clone())?;

    // calculate flow rewards
    let mut messages: Vec<CosmosMsg> = vec![];
    let mut flows: Vec<Flow> = FLOWS
        .may_load(deps.storage)?
        .unwrap_or_default()
        .into_iter()
        //.filter(|flow| flow.start_timestamp <= env.block.time.seconds())
        .filter(|flow| flow.start_epoch <= current_epoch)
        .collect();
    //
    // let user_last_claimed = LAST_CLAIMED_INDEX
    //     .may_load(deps.storage, info.sender.clone())?
    //     // .unwrap_or(env.block.time.seconds());
    //     .unwrap_or(0u64);

    for mut flow in flows.iter_mut() {
        // check if flow already ended and if everything has been claimed for that flow.
        if current_epoch > flow.end_epoch && flow.claimed_amount == flow.flow_asset.amount {
            // if so, skip flow.
            continue;
        }

        let mut last_user_weight_seen: (u64, Uint128) = (064, Uint128::zero());
        // check what is the earliest available weight for the user
        let first_available_weight_for_user = ADDRESS_WEIGHT_HISTORY
            .prefix(&info.sender.clone())
            .range(deps.storage, None, None, Order::Ascending)
            .take(1)
            .map(|item| Ok(item?))
            .collect::<StdResult<Vec<(u64, Uint128)>>>()?;

        if !first_available_weight_for_user.is_empty() {
            last_user_weight_seen = first_available_weight_for_user[0];
        }

        let last_claimed_epoch = LAST_CLAIMED_EPOCH.may_load(deps.storage, &info.sender.clone())?;
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

        for epoch_id in first_claimable_epoch..=current_epoch {
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

            // check if the flow is active in this epoch
            if epoch_id < flow.start_epoch {
                // the flow is not active at this epoch yet, skip
                continue;
            }

            let global_weight_at_epoch = GLOBAL_WEIGHT_SNAPSHOT
                .may_load(deps.storage, epoch_id)?
                .unwrap_or_default();

            if global_weight_at_epoch == Uint128::zero() {
                // Nothing to compute here as the global weight is 0, we don't want to divide by 0
                continue;
            }

            let user_share_at_epoch = Decimal256::from_ratio(user_weight, global_weight_at_epoch);

            // calculate emissions per epoch
            let emitted_tokens = if flow.emitted_tokens.is_empty() {
                Uint128::zero()
            } else {
                let previous_emission = flow
                    .emitted_tokens
                    .get(&(epoch_id - 1))
                    .unwrap_or(&Uint128::zero())
                    .clone();
                flow.emitted_tokens
                    .get(&epoch_id)
                    .unwrap_or(&previous_emission)
                    .clone()
            };

            // emission = (total_tokens - emitted_tokens_at_epoch) / (flow_start + flow_duration - epoch) = (total_tokens - emitted_tokens_at_epoch) / (flow_end - epoch)
            let emission_per_epoch = flow
                .flow_asset
                .amount
                .saturating_sub(emitted_tokens)
                .checked_div(Uint128::from(flow.end_epoch - epoch_id))?;

            // calculate user reward

            let user_reward = Uint256::from_uint128(emission_per_epoch) * user_share_at_epoch;

            let user_reward_at_epoch: Uint128 = user_reward.try_into()?;

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
        }

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

    //todo clean the ADDRESS_WEIGHT_HISTORY for previous epochs as they are not needed anymore

    LAST_CLAIMED_EPOCH.save(deps.storage, &info.sender.clone(), &current_epoch)?;

    // save the new flow state
    FLOWS.save(deps.storage, &flows)?;

    //todo to remove?
    LAST_CLAIMED_INDEX.save(deps.storage, info.sender.clone(), &env.block.time.seconds())?;

    Ok(messages)
}
