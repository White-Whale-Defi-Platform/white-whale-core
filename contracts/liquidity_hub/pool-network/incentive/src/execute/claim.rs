use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

use crate::error::ContractError;

pub fn claim(mut deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    Ok(Response::new().add_messages(crate::claim::claim(&mut deps, &env, &info)?))
}
