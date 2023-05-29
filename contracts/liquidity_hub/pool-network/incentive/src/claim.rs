use cosmwasm_std::{
    to_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal256, Deps, DepsMut, Env, MessageInfo,
    StdError, Uint128, Uint256, WasmMsg,
};

use white_whale::pool_network::{
    asset::AssetInfo,
    incentive::{Curve, Flow},
};

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
