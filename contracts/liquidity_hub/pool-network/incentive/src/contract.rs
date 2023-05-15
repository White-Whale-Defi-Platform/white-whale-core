#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, Uint128};
use cw2::{get_contract_version, set_contract_version};
use white_whale::pool_network::incentive::{
    Config, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};

use semver::Version;

use crate::error::ContractError;
use crate::error::ContractError::MigrateInvalidVersion;
use crate::state::{CONFIG, FLOWS, FLOW_COUNTER, GLOBAL_WEIGHT};
use crate::{execute, queries};

// version info for migration info
const CONTRACT_NAME: &str = "white_whale-incentive";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        factory_address: deps.api.addr_validate(&info.sender.into_string())?,
        lp_address: deps.api.addr_validate(&msg.lp_address.to_string())?,
    };

    CONFIG.save(deps.storage, &config)?;

    FLOW_COUNTER.save(deps.storage, &0)?;
    FLOWS.save(deps.storage, &Vec::new())?;

    GLOBAL_WEIGHT.save(deps.storage, &Uint128::zero())?;

    Ok(Response::default()
        .add_attributes(vec![
            ("action", "instantiate".to_string()),
            ("lp_address", config.factory_address.to_string()),
            ("lp_address", config.lp_address.to_string()),
        ])
        .set_data(to_binary(
            &white_whale::pool_network::incentive::InstantiateReplyCallback {
                lp_address: msg.lp_address,
            },
        )?))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::OpenFlow {
            start_timestamp,
            end_timestamp,
            curve,
            flow_asset,
        } => execute::open_flow(
            deps,
            env,
            info,
            start_timestamp,
            end_timestamp,
            curve,
            flow_asset,
        ),
        ExecuteMsg::CloseFlow { flow_id } => execute::close_flow(deps, info, flow_id),
        ExecuteMsg::OpenPosition {
            amount,
            unbonding_duration,
            receiver,
        } => execute::open_position(deps, env, info, amount, unbonding_duration, receiver),
        ExecuteMsg::ExpandPosition {
            amount,
            unbonding_duration,
            receiver,
        } => execute::expand_position(deps, env, info, amount, unbonding_duration, receiver),
        ExecuteMsg::ClosePosition { unbonding_duration } => {
            execute::close_position(deps, env, info, unbonding_duration)
        }
        ExecuteMsg::Withdraw {} => execute::withdraw(deps, env, info),
        ExecuteMsg::Claim {} => execute::claim(deps, env, info),
    }
}

/// Handles the queries to the incentive contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Config {} => Ok(to_binary(&queries::get_config(deps)?)?),
        QueryMsg::Flow { flow_id } => Ok(to_binary(&queries::get_flow(deps, flow_id)?)?),
        QueryMsg::Flows {} => Ok(to_binary(&queries::get_flows(deps)?)?),
        QueryMsg::Positions { address } => {
            Ok(to_binary(&queries::get_positions(deps, env, address)?)?)
        }
        QueryMsg::Rewards { address } => Ok(to_binary(&queries::get_rewards(deps, env, address)?)?),
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
