mod after_trade;

pub use after_trade::after_trade;

use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdError, StdResult};
use vault_network::vault::CallbackMsg;

pub fn callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: CallbackMsg,
) -> StdResult<Response> {
    // callback can only be called by contract
    if info.sender != env.contract.address {
        return Err(StdError::GenericErr {
            msg: "Attempt to call callback function outside contract".to_string(),
        });
    }

    match msg {
        CallbackMsg::AfterTrade { old_balance } => after_trade(deps, env, old_balance),
    }
}
