use cosmwasm_std::{
    ensure, entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;

use white_whale_std::incentive_manager::{
    Config, ExecuteMsg, IncentiveAction, InstantiateMsg, PositionAction, QueryMsg,
};
use white_whale_std::vault_manager::MigrateMsg;

use crate::error::ContractError;
use crate::helpers::validate_emergency_unlock_penalty;
use crate::state::{CONFIG, INCENTIVE_COUNTER};
use crate::{incentive, manager, position, queries};

const CONTRACT_NAME: &str = "white-whale_incentive-manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // ensure that max_concurrent_incentives is non-zero
    ensure!(
        msg.max_concurrent_incentives > 0,
        ContractError::UnspecifiedConcurrentIncentives
    );

    // ensure the unlocking duration range is valid
    ensure!(
        msg.max_unlocking_duration > msg.min_unlocking_duration,
        ContractError::InvalidUnlockingRange {
            min: msg.min_unlocking_duration,
            max: msg.max_unlocking_duration,
        }
    );

    let config = Config {
        epoch_manager_addr: deps.api.addr_validate(&msg.epoch_manager_addr)?,
        bonding_manager_addr: deps.api.addr_validate(&msg.bonding_manager_addr)?,
        create_incentive_fee: msg.create_incentive_fee,
        max_concurrent_incentives: msg.max_concurrent_incentives,
        max_incentive_epoch_buffer: msg.max_incentive_epoch_buffer,
        min_unlocking_duration: msg.min_unlocking_duration,
        max_unlocking_duration: msg.max_unlocking_duration,
        emergency_unlock_penalty: validate_emergency_unlock_penalty(msg.emergency_unlock_penalty)?,
    };

    CONFIG.save(deps.storage, &config)?;
    INCENTIVE_COUNTER.save(deps.storage, &0)?;
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(msg.owner.as_str()))?;

    Ok(Response::default().add_attributes(vec![
        ("action", "instantiate".to_string()),
        ("owner", msg.owner),
        ("epoch_manager_addr", config.epoch_manager_addr.to_string()),
        (
            "bonding_manager_addr",
            config.bonding_manager_addr.to_string(),
        ),
        ("create_flow_fee", config.create_incentive_fee.to_string()),
        (
            "max_concurrent_flows",
            config.max_concurrent_incentives.to_string(),
        ),
        (
            "max_flow_epoch_buffer",
            config.max_incentive_epoch_buffer.to_string(),
        ),
        (
            "min_unbonding_duration",
            config.min_unlocking_duration.to_string(),
        ),
        (
            "max_unbonding_duration",
            config.max_unlocking_duration.to_string(),
        ),
        (
            "emergency_unlock_penalty",
            config.emergency_unlock_penalty.to_string(),
        ),
    ]))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ManageIncentive { action } => match action {
            IncentiveAction::Fill { params } => {
                manager::commands::fill_incentive(deps, info, params)
            }
            IncentiveAction::Close {
                incentive_identifier,
            } => manager::commands::close_incentive(deps, info, incentive_identifier),
        },
        ExecuteMsg::UpdateOwnership(action) => {
            cw_utils::nonpayable(&info)?;
            white_whale_std::common::update_ownership(deps, env, info, action).map_err(Into::into)
        }
        ExecuteMsg::EpochChangedHook(msg) => {
            manager::commands::on_epoch_changed(deps, env, info, msg)
        }
        ExecuteMsg::Claim => incentive::commands::claim(deps, env, info),
        ExecuteMsg::ManagePosition { action } => match action {
            PositionAction::Fill {
                identifier,
                unlocking_duration,
                receiver,
            } => position::commands::fill_position(
                deps,
                &env,
                info,
                identifier,
                unlocking_duration,
                receiver,
            ),
            PositionAction::Close {
                identifier,
                lp_asset,
            } => position::commands::close_position(deps, env, info, identifier, lp_asset),
            PositionAction::Withdraw {
                identifier,
                emergency_unlock,
            } => {
                position::commands::withdraw_position(deps, env, info, identifier, emergency_unlock)
            }
        },
        ExecuteMsg::UpdateConfig {
            bonding_manager_addr,
            epoch_manager_addr,
            create_incentive_fee,
            max_concurrent_incentives,
            max_incentive_epoch_buffer,
            min_unlocking_duration,
            max_unlocking_duration,
            emergency_unlock_penalty,
        } => {
            cw_utils::nonpayable(&info)?;
            manager::commands::update_config(
                deps,
                info,
                bonding_manager_addr,
                epoch_manager_addr,
                create_incentive_fee,
                max_concurrent_incentives,
                max_incentive_epoch_buffer,
                min_unlocking_duration,
                max_unlocking_duration,
                emergency_unlock_penalty,
            )
        }
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Config => Ok(to_json_binary(&queries::query_manager_config(deps)?)?),
        QueryMsg::Ownership {} => Ok(to_json_binary(&cw_ownable::get_ownership(deps.storage)?)?),
        QueryMsg::Incentives {
            filter_by,
            start_after,
            limit,
        } => Ok(to_json_binary(&queries::query_incentives(
            deps,
            filter_by,
            start_after,
            limit,
        )?)?),
        QueryMsg::Positions {
            address,
            open_state,
        } => Ok(to_json_binary(&queries::query_positions(
            deps, address, open_state,
        )?)?),
        QueryMsg::Rewards { address } => Ok(to_json_binary(&queries::query_rewards(
            deps, &env, address,
        )?)?),
        QueryMsg::LPWeight {
            address,
            denom,
            epoch_id,
        } => Ok(to_json_binary(&queries::query_lp_weight(
            deps, address, denom, epoch_id,
        )?)?),
    }
}

#[cfg(not(tarpaulin_include))]
#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    use white_whale_std::migrate_guards::check_contract_name;

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
