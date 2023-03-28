#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult,
};
use cw2::{get_contract_version, set_contract_version};
use white_whale::pool_network::incentive_factory::{
    Config, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};

use semver::Version;

use crate::error::ContractError;
use crate::error::ContractError::MigrateInvalidVersion;
use crate::reply::create_incentive_reply::CREATE_INCENTIVE_REPLY_ID;
use crate::state::CONFIG;
use crate::{execute, queries, reply};

// version info for migration info
const CONTRACT_NAME: &str = "white_whale-incentive_factory";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // ensure that max_concurrent_flows is non-zero
    if msg.max_concurrent_flows == 0 {
        return Err(ContractError::UnspecifiedConcurrentFlows);
    }

    if msg.max_flow_start_time_buffer <= msg.min_unbonding_duration {
        return Err(ContractError::InvalidUnbondingRange {
            min: msg.min_unbonding_duration,
            max: msg.max_unbonding_duration,
        });
    }

    let config = Config {
        owner: deps.api.addr_canonicalize(info.sender.as_str())?,
        fee_collector_addr: deps
            .api
            .addr_canonicalize(msg.fee_collector_addr.as_str())?,
        create_flow_fee: msg.create_flow_fee,
        max_concurrent_flows: msg.max_concurrent_flows,
        incentive_code_id: msg.incentive_contract_id,
        max_flow_start_time_buffer: msg.max_flow_start_time_buffer,
        min_unbonding_duration: msg.min_unbonding_duration,
        max_unbonding_duration: msg.max_unbonding_duration,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    // Only the owner can execute messages on the factory
    let config: Config = CONFIG.load(deps.storage)?;
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    match msg {
        ExecuteMsg::CreateIncentive { lp_address } => {
            execute::create_incentive(deps, env, lp_address)
        }
        ExecuteMsg::UpdateConfig {
            fee_collector_addr,
            create_flow_fee,
            max_concurrent_flows,
            incentive_contract_id,
            max_flow_start_time_buffer,
        } => execute::update_config(
            deps,
            fee_collector_addr,
            create_flow_fee,
            max_concurrent_flows,
            incentive_contract_id,
            max_flow_start_time_buffer,
        ),
    }
}

/// Handles reply messages from submessages sent out by the incentive factory.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        CREATE_INCENTIVE_REPLY_ID => {
            reply::create_incentive_reply::create_incentive_reply(deps, msg)
        }
        id @ _ => Err(ContractError::UnknownReplyId { id }),
    }
}

/// Handles the queries to the incentive contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&queries::get_config(deps)?),
        QueryMsg::GetIncentive { lp_address } => {
            to_binary(&queries::get_incentive(deps, lp_address)?)
        }
    }
}

#[cfg(not(tarpaulin_include))]
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let version: Version = CONTRACT_VERSION.parse()?;
    let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;

    if storage_version >= version {
        return Err(MigrateInvalidVersion {
            current_version: storage_version,
            new_version: version,
        });
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
