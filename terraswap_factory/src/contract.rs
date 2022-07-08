use crate::{commands, queries};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError, StdResult,
};
use protobuf::Message;
use terraswap::asset::PairInfoRaw;
use terraswap::factory::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use terraswap::querier::query_pair_info_from_pair;

use crate::response::MsgInstantiateContractResponse;
use crate::state::{Config, CONFIG, PAIRS, TMP_PAIR_INFO};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let config = Config {
        owner: deps.api.addr_canonicalize(info.sender.as_str())?,
        token_code_id: msg.token_code_id,
        pair_code_id: msg.pair_code_id,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateConfig {
            owner,
            token_code_id,
            pair_code_id,
        } => commands::update_config(deps, env, info, owner, token_code_id, pair_code_id),
        ExecuteMsg::CreatePair {
            asset_infos,
            pool_fees,
        } => commands::create_pair(deps, env, info, asset_infos, pool_fees),
        ExecuteMsg::AddNativeTokenDecimals { denom, decimals } => {
            commands::add_native_token_decimals(deps, env, info, denom, decimals)
        }
    }
}

/// This just stores the result for future query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
    let tmp_pair_info = TMP_PAIR_INFO.load(deps.storage)?;

    let res: MsgInstantiateContractResponse =
        Message::parse_from_bytes(msg.result.unwrap().data.unwrap().as_slice()).map_err(|_| {
            StdError::parse_err("MsgInstantiateContractResponse", "failed to parse data")
        })?;

    let pair_contract = res.get_address();
    let pair_info = query_pair_info_from_pair(&deps.querier, Addr::unchecked(pair_contract))?;

    PAIRS.save(
        deps.storage,
        &tmp_pair_info.pair_key,
        &PairInfoRaw {
            liquidity_token: deps.api.addr_canonicalize(&pair_info.liquidity_token)?,
            contract_addr: deps.api.addr_canonicalize(pair_contract)?,
            asset_infos: tmp_pair_info.asset_infos,
            asset_decimals: tmp_pair_info.asset_decimals,
        },
    )?;

    Ok(Response::new().add_attributes(vec![
        ("pair_contract_addr", pair_contract),
        ("liquidity_token_addr", pair_info.liquidity_token.as_str()),
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
