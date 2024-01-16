use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint64,
};

use white_whale::fee_distributor::EpochResponse;

use crate::error::ContractError;
use crate::msg::InstantiateMsg;
use crate::state::CURRENT_EPOCH;

// use cw2::set_contract_version;

/*
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:fee-distributor-mock";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
*/

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    CURRENT_EPOCH.save(
        deps.storage,
        &white_whale::fee_distributor::Epoch {
            id: Uint64::one(),
            start_time: env.block.time,
            total: vec![],
            available: vec![],
            claimed: vec![],
            global_index: Default::default(),
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: white_whale::fee_distributor::ExecuteMsg,
) -> Result<Response, ContractError> {
    if let white_whale::fee_distributor::ExecuteMsg::NewEpoch {} = msg {
        CURRENT_EPOCH.update(deps.storage, |epoch| -> StdResult<_> {
            Ok(white_whale::fee_distributor::Epoch {
                id: epoch.id + Uint64::one(),
                start_time: epoch.start_time.plus_seconds(86400u64),
                total: vec![],
                available: vec![],
                claimed: vec![],
                global_index: Default::default(),
            })
        })?;
    }

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(
    deps: Deps,
    _env: Env,
    msg: white_whale::fee_distributor::QueryMsg,
) -> StdResult<Binary> {
    match msg {
        white_whale::fee_distributor::QueryMsg::Config {} => {}
        white_whale::fee_distributor::QueryMsg::CurrentEpoch {} => {
            return to_json_binary(&EpochResponse {
                epoch: CURRENT_EPOCH.load(deps.storage)?,
            });
        }
        white_whale::fee_distributor::QueryMsg::Epoch { .. } => {}
        white_whale::fee_distributor::QueryMsg::ClaimableEpochs { .. } => {}
        white_whale::fee_distributor::QueryMsg::Claimable { .. } => {}
    }

    to_json_binary(&"")
}
