use cosmwasm_std::{
    to_binary, BankMsg, Coin, CosmosMsg, Decimal256, DepsMut, Env, MessageInfo, Uint128, Uint256,
    WasmMsg,
};
use white_whale::pool_network::{asset::AssetInfo, incentive::Curve};

use crate::{
    error::ContractError,
    state::{ADDRESS_WEIGHT, FLOWS, GLOBAL_WEIGHT, LAST_CLAIMED_INDEX},
};

pub fn calculate_emission(
    current_time: u64,
    start: u64,
    end: u64,
    curve: &Curve,
    total: Uint128,
) -> Uint256 {
    let total = Uint256::from_uint128(total);

    let percentage = Decimal256::from_ratio(current_time - start, end - start);

    match curve {
        &Curve::Linear => percentage * total,
    }
}

/// Performs the claim function, returning all the [`CosmosMsg`]'s to run.
pub fn claim(
    deps: &mut DepsMut,
    env: &Env,
    info: &MessageInfo,
) -> Result<Vec<CosmosMsg>, ContractError> {
    // calculate user share
    let user_weight = ADDRESS_WEIGHT.load(deps.storage, info.sender.clone())?;
    let global_weight = GLOBAL_WEIGHT.load(deps.storage)?;

    let user_share = Decimal256::from_ratio(user_weight, global_weight);

    // calculate flow rewards
    let mut messages: Vec<CosmosMsg> = vec![];
    let mut flows = FLOWS.load(deps.storage)?;

    let user_last_claimed = LAST_CLAIMED_INDEX.may_load(deps.storage, info.sender.clone())?;

    for mut flow in flows.iter_mut() {
        let emissions = calculate_emission(
            env.block.time.seconds().max(flow.end_timestamp),
            user_last_claimed
                .unwrap_or_default()
                .max(flow.start_timestamp),
            flow.end_timestamp,
            &flow.curve,
            flow.flow_asset.amount,
        );
        let user_reward: Uint128 = (user_share * emissions).try_into()?;

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
