use cosmwasm_std::{entry_point, StdError};
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, SubMsg};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;

use white_whale::epoch_manager::{EpochV2, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use white_whale::migrate_guards::check_contract_name;

use crate::error::ContractError;
use crate::hooks::EpochChangedHookMsg;
use crate::state::{ADMIN, CONFIG, EPOCH, HOOKS};

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
        return Err(ContractError::Std(StdError::generic_err("start_time must be in the future")));
    }

    ADMIN.set(deps.branch(), Some(info.sender))?;
    EPOCH.save(deps.storage, &msg.start_epoch)?;
    Ok(Response::default())
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
            Ok(HOOKS.execute_add_hook(&ADMIN, deps, info, api.addr_validate(&contract_addr)?)?)
        }
        ExecuteMsg::RemoveHook { contract_addr } => {
            Ok(HOOKS.execute_remove_hook(&ADMIN, deps, info, api.addr_validate(&contract_addr)?)?)
        }
        ExecuteMsg::CreateEpoch {} => {
            let mut current_epoch = EPOCH.load(deps.storage)?;
            let config = CONFIG.load(deps.storage)?;

            if env
                .block
                .time
                .minus_nanos(current_epoch.start_time.nanos())
                .nanos()
                < config.epoch_config.duration.u64()
            {
                return Err(ContractError::CurrentEpochNotExpired {});
            }

            let mut current_epoch = EPOCH.load(deps.storage)?;
            current_epoch.id = current_epoch.checked_add(1u64)?;
            current_epoch.start_time = current_epoch.start_time.plus_nanos(config.epoch_config.duration.u64());

            EPOCH.save(deps.storage, &current_epoch)?;

            let messages = HOOKS.prepare_hooks(deps.storage, |hook| {
                EpochChangedHookMsg { current_epoch }
                    .clone()
                    .into_cosmos_msg(hook)
                    .map(SubMsg::new)
            })?;

            Ok(Response::default()
                .add_submessages(messages)
                .add_attributes(vec![
                    ("action", "create_epoch".to_string()),
                    ("current_epoch", current_epoch.to_string()),
                ]))
        }
        ExecuteMsg::UpdateConfig {
            owner,
            epoch_config,
        } => {}
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}

#[entry_point]
pub fn migrate(mut deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
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
