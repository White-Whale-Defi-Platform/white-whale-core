#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError, StdResult,
};
use cw2::{get_contract_version, set_contract_version};
use protobuf::Message;

use pool_network::asset::PairInfoRaw;
use pool_network::factory::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use pool_network::querier::query_pair_info_from_pair;
use semver::Version;

use crate::error::ContractError;
use crate::error::ContractError::MigrateInvalidVersion;
use crate::response::MsgInstantiateContractResponse;
use crate::state::{Config, CONFIG, PAIRS, TMP_PAIR_INFO};
use crate::{commands, queries};

// version info for migration info
const CONTRACT_NAME: &str = "white_whale-pool_factory";
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
        owner: deps.api.addr_canonicalize(info.sender.as_str())?,
        token_code_id: msg.token_code_id,
        pair_code_id: msg.pair_code_id,
        fee_collector_addr: deps.api.addr_validate(msg.fee_collector_addr.as_str())?,
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
        ExecuteMsg::UpdateConfig {
            owner,
            fee_collector_addr,
            token_code_id,
            pair_code_id,
        } => commands::update_config(deps, owner, fee_collector_addr, token_code_id, pair_code_id),
        ExecuteMsg::CreatePair {
            asset_infos,
            pool_fees,
            pair_type,
        } => commands::create_pair(deps, env, asset_infos, pool_fees, pair_type),
        ExecuteMsg::RemovePair { asset_infos } => commands::remove_pair(deps, env, asset_infos),
        ExecuteMsg::AddNativeTokenDecimals { denom, decimals } => {
            commands::add_native_token_decimals(deps, env, denom, decimals)
        }
        ExecuteMsg::MigratePair { contract, code_id } => {
            commands::execute_migrate_pair(deps, contract, code_id)
        }
        ExecuteMsg::UpdatePairConfig {
            pair_addr,
            owner,
            fee_collector_addr,
            pool_fees,
            feature_toggle,
        } => commands::update_pair_config(
            deps,
            pair_addr,
            owner,
            fee_collector_addr,
            pool_fees,
            feature_toggle,
        ),
    }
}

/// This just stores the result for future query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let tmp_pair_info = TMP_PAIR_INFO.load(deps.storage)?;

    let res: MsgInstantiateContractResponse =
        Message::parse_from_bytes(msg.result.unwrap().data.unwrap().as_slice()).map_err(|_| {
            StdError::parse_err("MsgInstantiateContractResponse", "failed to parse data")
        })?;

    let pair_contract = deps.api.addr_validate(&res.address)?;
    let pair_info = query_pair_info_from_pair(&deps.querier, pair_contract.clone())?;

    PAIRS.save(
        deps.storage,
        &tmp_pair_info.pair_key,
        &PairInfoRaw {
            liquidity_token: deps.api.addr_canonicalize(&pair_info.liquidity_token)?,
            contract_addr: deps.api.addr_canonicalize(pair_contract.as_str())?,
            asset_infos: tmp_pair_info.asset_infos,
            asset_decimals: tmp_pair_info.asset_decimals,
            pair_type: tmp_pair_info.pair_type,
        },
    )?;

    Ok(Response::new().add_attributes(vec![
        ("pair_contract_addr", pair_contract.as_str()),
        ("liquidity_token_addr", &pair_info.liquidity_token),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&queries::query_config(deps)?),
        QueryMsg::Pair { asset_infos } => to_binary(&queries::query_pair(deps, asset_infos)?),
        QueryMsg::Pairs { start_after, limit } => {
            to_binary(&queries::query_pairs(deps, start_after, limit)?)
        }
        QueryMsg::NativeTokenDecimals { denom } => {
            to_binary(&queries::query_native_token_decimal(deps, denom)?)
        }
    }
}

#[cfg(not(tarpaulin_include))]
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(mut deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    use crate::migrations;

    let version: Version = CONTRACT_VERSION.parse()?;
    let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;

    if storage_version >= version {
        return Err(MigrateInvalidVersion {
            current_version: storage_version,
            new_version: version,
        });
    }

    if storage_version <= Version::parse("1.0.8")? {
        migrations::migrate_to_v110(deps.branch())?;
    }
    if storage_version <= Version::parse("1.2.0")? {
        migrations::migrate_to_v120(deps.branch())?;
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
