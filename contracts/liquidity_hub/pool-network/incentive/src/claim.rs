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

pub fn calculate_emission(current_time: u64, start: u64, end: u64, flow: &Flow) -> Uint256 {
    let total = Uint256::from_uint128(flow.claimed_amount);

    let percentage = Decimal256::from_ratio(current_time - start, end - start);

    match &flow.curve {
        &Curve::Linear => percentage * total,
    }
}

pub fn get_user_share(deps: &Deps, address: Addr) -> Result<Decimal256, StdError> {
    // calculate user share
    let user_weight = ADDRESS_WEIGHT.load(deps.storage, address)?;
    let global_weight = GLOBAL_WEIGHT.load(deps.storage)?;

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
        user_last_claimed,
        flow.end_timestamp,
        &flow,
    );

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
    let mut flows = FLOWS.load(deps.storage)?;

    let user_last_claimed = LAST_CLAIMED_INDEX
        .may_load(deps.storage, info.sender.clone())?
        .unwrap_or(env.block.time.seconds());

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
