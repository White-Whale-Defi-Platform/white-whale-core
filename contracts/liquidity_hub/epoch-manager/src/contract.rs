use cosmwasm_std::{entry_point, to_json_binary, StdError};
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;

use white_whale::epoch_manager::epoch_manager::{
    Config, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};
use white_whale::migrate_guards::check_contract_name;

use crate::error::ContractError;
use crate::state::{ADMIN, CONFIG, EPOCH};
use crate::{commands, queries};

// version info for migration info
const CONTRACT_NAME: &str = "white_whale-epoch-manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // validate start_time for the initial epoch
    if msg.start_epoch.start_time < env.block.time {
        return Err(ContractError::Std(StdError::generic_err(
            "start_time must be in the future",
        )));
    }

    if msg.epoch_config.genesis_epoch.u64() != msg.start_epoch.start_time.nanos() {
        return Err(ContractError::Std(StdError::generic_err(
            "genesis_epoch must be equal to start_epoch.start_time",
        )));
    }

    ADMIN.set(deps.branch(), Some(info.sender))?;
    EPOCH.save(deps.storage, &msg.start_epoch)?;
    CONFIG.save(
        deps.storage,
        &Config {
            epoch_config: msg.epoch_config.clone(),
        },
    )?;
    Ok(Response::default().add_attributes(vec![
        ("action", "instantiate".to_string()),
        ("start_epoch", msg.start_epoch.to_string()),
        ("epoch_config", msg.epoch_config.to_string()),
    ]))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let api = deps.api;
    match msg {
        ExecuteMsg::AddHook { contract_addr } => {
            commands::add_hook(deps, info, api, &contract_addr)
        }
        ExecuteMsg::RemoveHook { contract_addr } => {
            commands::remove_hook(deps, info, api, &contract_addr)
        }
        ExecuteMsg::CreateEpoch {} => commands::create_epoch(deps, env),
        ExecuteMsg::UpdateConfig {
            owner,
            epoch_config,
        } => commands::update_config(deps, &info, owner, epoch_config),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => Ok(to_json_binary(&queries::query_config(deps)?)?),
        QueryMsg::CurrentEpoch {} => Ok(to_json_binary(&queries::query_current_epoch(deps)?)?),
        QueryMsg::Epoch { id } => Ok(to_json_binary(&queries::query_epoch(deps, id)?)?),
    }
}

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    check_contract_name(deps.storage, CONTRACT_NAME.to_string())?;
    let version: Version = CONTRACT_VERSION.parse()?;
    let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;

    if storage_version >= version {
        return Err(ContractError::MigrateInvalidVersion {
            current_version: storage_version,
            new_version: version,
        });
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}
