use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;

use white_whale::incentive_manager::{
    Config, ExecuteMsg, IncentiveAction, InstantiateMsg, QueryMsg,
};
use white_whale::vault_manager::MigrateMsg;

use crate::error::ContractError;
use crate::manager;
use crate::state::CONFIG;

const CONTRACT_NAME: &str = "crates.io:incentive-manager";
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
    if msg.max_concurrent_incentives == 0 {
        return Err(ContractError::UnspecifiedConcurrentIncentives);
    }

    if msg.max_unbonding_duration < msg.min_unbonding_duration {
        return Err(ContractError::InvalidUnbondingRange {
            min: msg.min_unbonding_duration,
            max: msg.max_unbonding_duration,
        });
    }

    let config = Config {
        epoch_manager_addr: deps.api.addr_validate(&msg.epoch_manager_addr)?,
        whale_lair_addr: deps.api.addr_validate(&msg.whale_lair_addr)?,
        create_incentive_fee: msg.create_incentive_fee,
        max_concurrent_incentives: msg.max_concurrent_incentives,
        max_incentive_epoch_buffer: msg.max_incentive_epoch_buffer,
        min_unbonding_duration: msg.min_unbonding_duration,
        max_unbonding_duration: msg.max_unbonding_duration,
    };

    CONFIG.save(deps.storage, &config)?;

    cw_ownable::initialize_owner(deps.storage, deps.api, Some(msg.owner.as_str()))?;

    Ok(Response::default().add_attributes(vec![
        ("action", "instantiate".to_string()),
        ("owner", msg.owner.to_string()),
        ("epoch_manager_addr", config.epoch_manager_addr.to_string()),
        ("whale_lair_addr", config.whale_lair_addr.to_string()),
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
            config.min_unbonding_duration.to_string(),
        ),
        (
            "max_unbonding_duration",
            config.max_unbonding_duration.to_string(),
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
            IncentiveAction::Create { params } => {
                manager::commands::create_incentive(deps, env, info, params)
            }
            IncentiveAction::Close {
                incentive_identifier,
            } => manager::commands::close_incentive(deps, info, incentive_identifier),
            IncentiveAction::Extend { params } => {
                manager::commands::expand_incentive(deps, env, info, params)
            }
        },
        ExecuteMsg::UpdateOwnership(action) => {
            Ok(
                cw_ownable::update_ownership(deps, &env.block, &info.sender, action).map(
                    |ownership| {
                        Response::default()
                            .add_attribute("action", "update_ownership")
                            .add_attributes(ownership.into_attributes())
                    },
                )?,
            )
        }
        ExecuteMsg::EpochChangedHook(msg) => {
            manager::commands::on_epoch_changed(deps, env, info, msg)
        }
    }
}

#[entry_point]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}

#[cfg(not(tarpaulin_include))]
#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    use white_whale::migrate_guards::check_contract_name;

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
