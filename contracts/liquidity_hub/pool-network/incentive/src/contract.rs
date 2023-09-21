#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, Uint128, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};
use white_whale::pool_network::incentive::{
    Config, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};

use semver::Version;
use white_whale::migrate_guards::check_contract_name;
use white_whale::pool_network::asset::AssetInfo;

use crate::error::ContractError;
use crate::error::ContractError::MigrateInvalidVersion;
use crate::state::{CONFIG, FLOW_COUNTER, GLOBAL_WEIGHT};
use crate::{execute, migrations, queries};

// version info for migration info
const CONTRACT_NAME: &str = "white_whale-incentive";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    match msg.lp_asset.clone() {
        AssetInfo::Token { contract_addr } => {
            deps.api.addr_validate(&contract_addr)?;
        }
        AssetInfo::NativeToken { .. } => {}
    };

    let config = Config {
        factory_address: deps.api.addr_validate(&info.sender.into_string())?,
        fee_distributor_address: deps.api.addr_validate(&msg.fee_distributor_address)?,
        lp_asset: msg.lp_asset.clone(),
    };

    CONFIG.save(deps.storage, &config)?;
    FLOW_COUNTER.save(deps.storage, &0)?;
    GLOBAL_WEIGHT.save(deps.storage, &Uint128::zero())?;

    Ok(Response::default()
        .add_attributes(vec![
            ("action", "instantiate".to_string()),
            ("factory_address", config.factory_address.to_string()),
            (
                "fee_distributor_address",
                config.fee_distributor_address.to_string(),
            ),
            ("lp_asset", config.lp_asset.to_string()),
        ])
        .set_data(to_binary(
            &white_whale::pool_network::incentive::InstantiateReplyCallback {
                lp_asset: msg.lp_asset,
            },
        )?)
        // takes a snapshot of the global weight at the current epoch from the start
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_binary(&ExecuteMsg::TakeGlobalWeightSnapshot {})?,
            funds: vec![],
        })))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::TakeGlobalWeightSnapshot {} => execute::take_global_weight_snapshot(deps),
        ExecuteMsg::OpenFlow {
            start_epoch,
            end_epoch,
            curve,
            flow_asset,
            flow_label,
        } => execute::open_flow(
            deps,
            env,
            info,
            start_epoch,
            end_epoch,
            curve,
            flow_asset,
            flow_label,
        ),
        ExecuteMsg::CloseFlow { flow_identifier } => {
            execute::close_flow(deps, info, flow_identifier)
        }
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
        ExecuteMsg::Claim {} => execute::claim(deps, info),
        ExecuteMsg::ExpandFlow {
            flow_identifier,
            end_epoch,
            flow_asset,
        } => execute::expand_flow(deps, info, env, flow_identifier, end_epoch, flow_asset),
    }
}

/// Handles the queries to the incentive contract.
#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Config {} => Ok(to_binary(&queries::get_config(deps)?)?),
        QueryMsg::Flow {
            flow_identifier,
            start_epoch,
            end_epoch,
        } => Ok(to_binary(&queries::get_flow(
            deps,
            flow_identifier,
            start_epoch,
            end_epoch,
        )?)?),
        QueryMsg::Flows {
            start_epoch,
            end_epoch,
        } => Ok(to_binary(&queries::get_flows(
            deps,
            start_epoch,
            end_epoch,
        )?)?),
        QueryMsg::Positions { address } => {
            Ok(to_binary(&queries::get_positions(deps, env, address)?)?)
        }
        QueryMsg::Rewards { address } => Ok(to_binary(&queries::get_rewards(deps, address)?)?),
        QueryMsg::GlobalWeight { epoch_id } => {
            Ok(to_binary(&queries::get_global_weight(deps, epoch_id)?)?)
        }
        QueryMsg::CurrentEpochRewardsShare { address } => Ok(to_binary(
            &queries::get_rewards_share(deps, deps.api.addr_validate(&address)?)?,
        )?),
    }
}

#[cfg(not(tarpaulin_include))]
#[entry_point]
pub fn migrate(mut deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    check_contract_name(deps.storage, CONTRACT_NAME.to_string())?;

    let version: Version = CONTRACT_VERSION.parse()?;
    let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;

    if storage_version >= version {
        return Err(MigrateInvalidVersion {
            current_version: storage_version,
            new_version: version,
        });
    }

    if storage_version < Version::parse("1.0.4")? {
        migrations::migrate_to_v106(deps.branch())?;
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
