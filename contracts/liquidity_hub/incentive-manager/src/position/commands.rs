use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

use white_whale::incentive_manager::PositionParams;

use crate::ContractError;

pub(crate) fn fill_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    params: PositionParams,
) -> Result<Response, ContractError> {
    Ok(Response::default().add_attributes(vec![("action", "fill_position".to_string())]))
}

pub(crate) fn close_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    unbonding_duration: u64,
) -> Result<Response, ContractError> {
    Ok(Response::default().add_attributes(vec![("action", "close_position".to_string())]))
}
