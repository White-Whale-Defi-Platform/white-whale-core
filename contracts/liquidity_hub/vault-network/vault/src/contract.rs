use cosmwasm_std::{entry_point, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use vault_network::vault::{ExecuteMsg, InstantiateMsg};

use crate::{
    execute::{callback, deposit, flash_loan, update_config, withdraw},
    state::{Config, CONFIG},
};

const CONTRACT_NAME: &str = "vault_factory";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: deps.api.addr_validate(&msg.owner)?,
        asset_info: msg.asset_info,

        deposit_enabled: true,
        flash_loan_enabled: true,
        withdraw_enabled: true,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateConfig {
            flash_loan_enabled,
            withdraw_enabled,
            deposit_enabled,
            new_owner,
        } => update_config(
            deps,
            flash_loan_enabled,
            withdraw_enabled,
            deposit_enabled,
            new_owner,
        ),
        ExecuteMsg::Deposit { amount } => deposit(deps, env, info, amount),
        ExecuteMsg::Withdraw { amount } => withdraw(deps, info, amount),
        ExecuteMsg::FlashLoan { amount, msg } => flash_loan(deps, env, info, amount, msg),
        ExecuteMsg::Callback(msg) => callback(deps, env, info, msg),
    }
}
