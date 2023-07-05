use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use white_whale::vault_manager::{ManagerConfig, ExecuteMsg, InstantiateMsg, QueryMsg};
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::state::MANAGER_CONFIG;

// version info for migration info
const CONTRACT_NAME: &str = "ww-vault-manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let manager_config = ManagerConfig {
        owner: deps.api.addr_validate(&msg.owner)?,
        token_id: msg.token_id,
        fee_collector_addr: deps.api.addr_validate(&msg.fee_collector_addr)?,
        flash_loan_enabled: true,
        deposit_enabled: true,
        withdraw_enabled: true,
    };
    MANAGER_CONFIG.save(deps.storage, &manager_config)?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateVault { .. } => {}
        ExecuteMsg::RemoveVault { .. } => {}
        ExecuteMsg::UpdateVault { .. } => {}
        ExecuteMsg::UpdateManagerConfig { .. } => {}
        ExecuteMsg::Deposit { .. } => {}
        ExecuteMsg::Withdraw { .. } => {}
        ExecuteMsg::Receive(_) => {}
        ExecuteMsg::Callback(_) => {}
        ExecuteMsg::FlashLoan { .. } => {}
        ExecuteMsg::NextLoan { .. } => {}
        ExecuteMsg::CompleteLoan { .. } => {}
    }

    Ok(Response::default())
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}
