use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

use crate::error::ContractError;

pub fn claim(mut deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    Ok(Response::default()
        .add_attributes(vec![("action", "claim")])
        .add_messages(crate::claim::claim(&mut deps, &env, &info)?))
}
