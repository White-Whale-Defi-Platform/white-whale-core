use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdError, Uint128, WasmMsg};
use white_whale::pool_network::incentive::OpenPosition;

use crate::{
    error::ContractError,
    funds_validation::validate_funds_sent,
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
    let incentive_factory_config: white_whale::pool_network::incentive_factory::ConfigResponse =
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
    let transfer_token_msg = validate_funds_sent(&deps.as_ref(), env, lp_token, info, amount)?;

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

    Ok(Response::new().add_message(transfer_token_msg))
}
