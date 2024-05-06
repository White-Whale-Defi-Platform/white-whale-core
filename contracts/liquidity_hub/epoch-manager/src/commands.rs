use cosmwasm_std::{Api, DepsMut, Env, MessageInfo, Response, SubMsg};

use white_whale_std::epoch_manager::epoch_manager::EpochConfig;
use white_whale_std::epoch_manager::hooks::EpochChangedHookMsg;

use crate::queries::query_current_epoch;
use crate::state::{ADMIN, CONFIG, EPOCHS, HOOKS};
use crate::ContractError;

/// Adds a new hook to the contract.
pub fn add_hook(
    deps: DepsMut,
    info: MessageInfo,
    api: &dyn Api,
    contract_addr: &str,
) -> Result<Response, ContractError> {
    Ok(HOOKS.execute_add_hook(&ADMIN, deps, info, api.addr_validate(contract_addr)?)?)
}

pub(crate) fn remove_hook(
    deps: DepsMut,
    info: MessageInfo,
    api: &dyn Api,
    contract_addr: &str,
) -> Result<Response, ContractError> {
    Ok(HOOKS.execute_remove_hook(&ADMIN, deps, info, api.addr_validate(contract_addr)?)?)
}

/// Creates a new epoch.
pub fn create_epoch(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    cw_utils::nonpayable(&info)?;

    let mut current_epoch = query_current_epoch(deps.as_ref())?.epoch;
    let config = CONFIG.load(deps.storage)?;
    println!("Creating new epoch");

    println!("Current epoch: {:?}", current_epoch);
    println!(
        "env
        .block
        .time: {:?}",
        env.block.time
    );

    if env
        .block
        .time
        .minus_nanos(current_epoch.start_time.nanos())
        .nanos()
        < config.epoch_config.duration.u64()
    {
        return Err(ContractError::CurrentEpochNotExpired);
    }
    current_epoch.id = current_epoch
        .id
        .checked_add(1u64)
        .ok_or(ContractError::EpochOverflow)?;
    current_epoch.start_time = current_epoch
        .start_time
        .plus_nanos(config.epoch_config.duration.u64());

    EPOCHS.save(deps.storage, current_epoch.id, &current_epoch)?;

    let messages = HOOKS.prepare_hooks(deps.storage, |hook| {
        EpochChangedHookMsg {
            current_epoch: current_epoch.clone(),
        }
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

/// Updates the config of the contract.
pub fn update_config(
    mut deps: DepsMut,
    info: &MessageInfo,
    owner: Option<String>,
    epoch_config: Option<EpochConfig>,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    if let Some(owner) = owner.clone() {
        let new_admin = deps.api.addr_validate(owner.as_str())?;
        ADMIN.set(deps.branch(), Some(new_admin))?;
    }

    let mut config = CONFIG.load(deps.storage)?;

    if let Some(epoch_config) = epoch_config.clone() {
        config.epoch_config = epoch_config;
        CONFIG.save(deps.storage, &config)?;
    }

    Ok(Response::default().add_attributes(vec![
        ("action", "update_config".to_string()),
        ("owner", owner.unwrap_or_else(|| info.sender.to_string())),
        (
            "epoch_config",
            epoch_config.unwrap_or(config.epoch_config).to_string(),
        ),
    ]))
}
