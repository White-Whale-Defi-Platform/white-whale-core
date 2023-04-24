use cosmwasm_std::{to_binary, DepsMut, Env, MessageInfo, Response, StdError, Uint128, WasmMsg};
use white_whale::pool_network::incentive::OpenPosition;

use crate::{
    error::ContractError,
    state::{ADDRESS_WEIGHT, CONFIG, GLOBAL_WEIGHT, OPEN_POSITIONS},
    weight::calculate_weight,
};

pub fn open_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
    unbonding_duration: u64,
    receiver: Option<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let lp_token = deps.api.addr_humanize(&config.lp_address)?;
    let factory_address = deps.api.addr_humanize(&config.factory_address)?;

    // validate unbonding duration
    let incentive_factory_config: white_whale::pool_network::incentive_factory::GetConfigResponse =
        deps.querier.query_wasm_smart(
            factory_address,
            &white_whale::pool_network::incentive_factory::QueryMsg::Config {},
        )?;

    if unbonding_duration < incentive_factory_config.min_unbonding_duration
        || unbonding_duration > incentive_factory_config.max_unbonding_duration
    {
        return Err(ContractError::InvalidUnbondingDuration {
            min: incentive_factory_config.min_unbonding_duration,
            max: incentive_factory_config.max_unbonding_duration,
            specified: unbonding_duration,
        });
    }

    // if receiver was not specified, default to the sender of the message.
    let receiver = receiver
        .map(|r| deps.api.addr_validate(&r))
        .unwrap_or_else(|| Ok(info.sender.clone()))?;

    // ensure that user gave us an allowance for the token amount
    let allowance: cw20::AllowanceResponse = deps.querier.query_wasm_smart(
        lp_token.clone(),
        &cw20::Cw20QueryMsg::Allowance {
            owner: info.sender.clone().into_string(),
            spender: env.contract.address.clone().into_string(),
        },
    )?;

    if allowance.allowance < amount {
        return Err(ContractError::MissingPositionDeposit {
            allowance_amount: allowance.allowance,
            deposited_amount: amount,
        });
    }

    // send the lp deposit to us
    let messages = vec![WasmMsg::Execute {
        contract_addr: lp_token.into_string(),
        msg: to_binary(&cw20::Cw20ExecuteMsg::TransferFrom {
            owner: info.sender.into_string(),
            recipient: env.contract.address.into_string(),
            amount,
        })?,
        funds: vec![],
    }];

    // ensure an existing position with this unbonding time doesn't exist
    let existing_position = OPEN_POSITIONS
        .may_load(deps.storage, receiver.clone())?
        .unwrap_or_default()
        .into_iter()
        .find(|position| position.unbonding_duration == unbonding_duration);
    if existing_position.is_some() {
        return Err(ContractError::DuplicatePosition);
    }

    // todo: claim??

    // create the new position
    OPEN_POSITIONS.update::<_, StdError>(deps.storage, receiver.clone(), |positions| {
        let mut positions = positions.unwrap_or_default();

        positions.push(OpenPosition {
            amount,
            unbonding_duration,
        });

        Ok(positions)
    })?;

    // add the weight
    let weight = calculate_weight(unbonding_duration, amount)?;
    GLOBAL_WEIGHT.update::<_, StdError>(deps.storage, |global_weight| {
        Ok(global_weight.checked_add(weight)?)
    })?;
    ADDRESS_WEIGHT.update::<_, StdError>(deps.storage, receiver, |user_weight| {
        Ok(user_weight.unwrap_or_default().checked_add(weight)?)
    })?;

    Ok(Response::new().add_messages(messages))
}
