use cosmwasm_std::{entry_point, DepsMut, Env, Reply, Response, StdError, StdResult};
use vault_network::vault::INSTANTIATE_LP_TOKEN_REPLY_ID;

mod lp_instantiate;

use lp_instantiate::lp_instantiate;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
    match msg.id {
        _ if msg.id == INSTANTIATE_LP_TOKEN_REPLY_ID => lp_instantiate(deps, msg),
        _ => Err(StdError::generic_err(format!(
            "Did not handle message reply of id '{}'",
            msg.id
        ))),
    }
}
