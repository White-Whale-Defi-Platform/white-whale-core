use crate::error::ContractError;
use crate::helpers;
use crate::state::{
    get_protocol_fees_for_asset, ConfigResponse, ALL_TIME_COLLECTED_PROTOCOL_FEES,
    COLLECTED_PROTOCOL_FEES, CONFIG, PAIR_INFO,
};
use cosmwasm_std::{Deps, Uint128};
use terraswap::asset::{Asset, PairInfo, PairInfoRaw};
use terraswap::pair::{
    PoolResponse, ProtocolFeesResponse, ReverseSimulationResponse, SimulationResponse,
};
use terraswap::querier::query_token_info;

/// Queries the [PairInfo] of the pool
pub fn query_pair_info(deps: Deps) -> Result<PairInfo, ContractError> {
    let pair_info: PairInfoRaw = PAIR_INFO.load(deps.storage)?;
    let pair_info = pair_info.to_normal(deps.api)?;

    Ok(pair_info)
}

/// Queries the Pool info, i.e. Assets and total share
pub fn query_pool(deps: Deps) -> Result<PoolResponse, ContractError> {
    let pair_info: PairInfoRaw = PAIR_INFO.load(deps.storage)?;
    let contract_addr = deps.api.addr_humanize(&pair_info.contract_addr)?;
    let assets: [Asset; 2] = pair_info.query_pools(&deps.querier, deps.api, contract_addr)?;
    let total_share: Uint128 = query_token_info(
        &deps.querier,
        deps.api.addr_humanize(&pair_info.liquidity_token)?,
    )?
    .total_supply;

    let resp = PoolResponse {
        assets,
        total_share,
    };

    Ok(resp)
}

/// Queries a swap simulation. Used to know how much the target asset will be returned for the source token
pub fn query_simulation(
    deps: Deps,
    offer_asset: Asset,
) -> Result<SimulationResponse, ContractError> {
    let pair_info: PairInfoRaw = PAIR_INFO.load(deps.storage)?;

    let contract_addr = deps.api.addr_humanize(&pair_info.contract_addr)?;
    let pools: [Asset; 2] = pair_info.query_pools(&deps.querier, deps.api, contract_addr)?;

    let offer_pool: Asset;
    let ask_pool: Asset;
    if offer_asset.info.equal(&pools[0].info) {
        offer_pool = pools[0].clone();
        ask_pool = pools[1].clone();
    } else if offer_asset.info.equal(&pools[1].info) {
        offer_pool = pools[1].clone();
        ask_pool = pools[0].clone();
    } else {
        return Err(ContractError::AssetMismatch {});
    }

    let pool_fees = CONFIG.load(deps.storage)?.pool_fees;

    let swap_computation = helpers::compute_swap(
        offer_pool.amount,
        ask_pool.amount,
        offer_asset.amount,
        pool_fees,
    );

    Ok(SimulationResponse {
        return_amount: swap_computation.return_amount,
        spread_amount: swap_computation.spread_amount,
        swap_fee_amount: swap_computation.swap_fee_amount,
        protocol_fee_amount: swap_computation.protocol_fee_amount,
    })
}

/// Queries a swap reverse simulation. Used to derive the number of source tokens returned for
/// the number of target tokens.
pub fn query_reverse_simulation(
    deps: Deps,
    ask_asset: Asset,
) -> Result<ReverseSimulationResponse, ContractError> {
    let pair_info: PairInfoRaw = PAIR_INFO.load(deps.storage)?;

    let contract_addr = deps.api.addr_humanize(&pair_info.contract_addr)?;
    let pools: [Asset; 2] = pair_info.query_pools(&deps.querier, deps.api, contract_addr)?;

    let offer_pool: Asset;
    let ask_pool: Asset;
    if ask_asset.info.equal(&pools[0].info) {
        ask_pool = pools[0].clone();
        offer_pool = pools[1].clone();
    } else if ask_asset.info.equal(&pools[1].info) {
        ask_pool = pools[1].clone();
        offer_pool = pools[0].clone();
    } else {
        return Err(ContractError::AssetMismatch {});
    }

    let pool_fees = CONFIG.load(deps.storage)?.pool_fees;
    let offer_amount_computation = helpers::compute_offer_amount(
        offer_pool.amount,
        ask_pool.amount,
        ask_asset.amount,
        pool_fees,
    );

    Ok(ReverseSimulationResponse {
        offer_amount: offer_amount_computation.offer_amount,
        spread_amount: offer_amount_computation.spread_amount,
        swap_fee_amount: offer_amount_computation.swap_fee_amount,
        protocol_fee_amount: offer_amount_computation.protocol_fee_amount,
    })
}

/// Queries the [Config], which contains the owner, pool_fees and feature_toggle
pub fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}

/// Queries the protocol fees on the pool
pub fn query_protocol_fees(
    deps: Deps,
    asset_id: Option<String>,
    all_time: Option<bool>,
) -> Result<ProtocolFeesResponse, ContractError> {
    if let Some(all_time) = all_time {
        if all_time {
            let fees = ALL_TIME_COLLECTED_PROTOCOL_FEES.load(deps.storage)?;
            return Ok(ProtocolFeesResponse { fees });
        }
    }

    if let Some(asset_id) = asset_id {
        let fee = get_protocol_fees_for_asset(deps.storage, asset_id)?;
        return Ok(ProtocolFeesResponse { fees: vec![fee] });
    }

    let fees = COLLECTED_PROTOCOL_FEES.load(deps.storage)?;
    Ok(ProtocolFeesResponse { fees })
}
