mod vault_instantiate;

use cosmwasm_std::{entry_point, DepsMut, Env, Reply, Response, StdError};
pub use vault_instantiate::vault_instantiate;
use vault_network::vault_factory::INSTANTIATE_VAULT_REPLY_ID;

use crate::err::{StdResult, VaultFactoryError};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> StdResult<Response> {
    match msg.id {
        _ if msg.id == INSTANTIATE_VAULT_REPLY_ID => vault_instantiate(deps, env, msg),
        _ => Err(VaultFactoryError::Std(StdError::generic_err(format!(
            "Did not handle message reply of id '{0}'",
            msg.id
        )))),
    }
}
