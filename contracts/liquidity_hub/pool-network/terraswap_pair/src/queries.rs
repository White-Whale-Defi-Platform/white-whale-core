use cosmwasm_std::{Deps, StdResult, Uint128};
use cw_storage_plus::Item;

use terraswap::asset::{Asset, PairInfo, PairInfoRaw};
use terraswap::pair::{
    ConfigResponse, PoolResponse, ProtocolFeesResponse, ReverseSimulationResponse,
    SimulationResponse,
};
use terraswap::querier::query_token_info;

use crate::error::ContractError;
use crate::helpers;
use crate::helpers::get_protocol_fee_for_asset;
use crate::state::{get_fees_for_asset, COLLECTED_PROTOCOL_FEES, CONFIG, PAIR_INFO};

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

    let collected_protocol_fees = COLLECTED_PROTOCOL_FEES.load(deps.storage)?;
    let assets = pair_info
        .query_pools(&deps.querier, deps.api, contract_addr)?
        .iter()
        .map(|asset| {
            // deduct protocol fee for that asset
            let protocol_fee =
                get_protocol_fee_for_asset(collected_protocol_fees.clone(), asset.clone().get_id());

            Asset {
                info: asset.info.clone(),
                amount: asset.amount - protocol_fee,
            }
        })
        .collect();

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

    let offer_pool: Asset;
    let ask_pool: Asset;
    let collected_protocol_fees = COLLECTED_PROTOCOL_FEES.load(deps.storage)?;

    // To calculate pool amounts properly we should subtract the protocol fees from the pool
    let pools = pair_info
        .query_pools(&deps.querier, deps.api, contract_addr)?
        .into_iter()
        .map(|mut pool| {
            // subtract the protocol fee from the pool
            let protocol_fee =
                get_protocol_fee_for_asset(collected_protocol_fees.clone(), pool.clone().get_id());
            pool.amount = pool.amount.checked_sub(protocol_fee)?;

            Ok(pool)
        })
        .collect::<StdResult<Vec<_>>>()?;

    if offer_asset.info.equal(&pools[0].info) {
        offer_pool = pools[0].clone();
        ask_pool = pools[1].clone();
    } else if offer_asset.info.equal(&pools[1].info) {
        offer_pool = pools[1].clone();
        ask_pool = pools[0].clone();
    } else {
        return Err(ContractError::AssetMismatch {});
    }

    let config = CONFIG.load(deps.storage)?;
    let pool_fees = config.pool_fees;

    let swap_computation = helpers::compute_swap(
        offer_pool,
        ask_pool,
        offer_asset.amount,
        pool_fees,
        config.collect_protocol_fees_in,
    )?;

    Ok(SimulationResponse {
        return_amount: swap_computation.return_asset.amount,
        spread_amount: swap_computation.spread_amount,
        swap_fee_amount: swap_computation.swap_fee_asset.amount,
        protocol_fee_amount: swap_computation.protocol_fee_asset.amount,
        burn_fee_amount: swap_computation.burn_fee_asset.amount,
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

    // To calculate pool amounts properly we should subtract the protocol fees from the pool
    let collected_protocol_fees = COLLECTED_PROTOCOL_FEES.load(deps.storage)?;

    let pools = pair_info
        .query_pools(&deps.querier, deps.api, contract_addr)?
        .into_iter()
        .map(|mut pool| {
            // subtract the protocol fee from the pool
            let protocol_fee =
                get_protocol_fee_for_asset(collected_protocol_fees.clone(), pool.clone().get_id());
            pool.amount = pool.amount.checked_sub(protocol_fee)?;

            Ok(pool)
        })
        .collect::<StdResult<Vec<_>>>()?;

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

    let config = CONFIG.load(deps.storage)?;
    let pool_fees = config.pool_fees;
    let collect_protocol_fees_in = config.collect_protocol_fees_in;
    let offer_amount_computation = helpers::compute_offer_amount(
        offer_pool,
        ask_pool,
        ask_asset.amount,
        pool_fees,
        collect_protocol_fees_in,
    )?;

    Ok(ReverseSimulationResponse {
        offer_amount: offer_amount_computation.offer_amount,
        spread_amount: offer_amount_computation.spread_amount,
        swap_fee_amount: offer_amount_computation.swap_fee_amount,
        protocol_fee_amount: offer_amount_computation.protocol_fee_amount,
        burn_fee_amount: offer_amount_computation.burn_fee_amount,
    })
}

/// Queries the [Config], which contains the owner, pool_fees and feature_toggle
pub fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}

/// Queries the fees on the pool for the given fees_storage_item
pub fn query_fees(
    deps: Deps,
    asset_id: Option<String>,
    all_time: Option<bool>,
    fees_storage_item: Item<Vec<Asset>>,
    all_time_fees_storage_item: Option<Item<Vec<Asset>>>,
) -> Result<ProtocolFeesResponse, ContractError> {
    if let (Some(all_time), Some(all_time_fees_storage_item)) =
        (all_time, all_time_fees_storage_item)
    {
        if all_time {
            let fees = all_time_fees_storage_item.load(deps.storage)?;
            return Ok(ProtocolFeesResponse { fees });
        }
    }

    if let Some(asset_id) = asset_id {
        let fee = get_fees_for_asset(deps.storage, asset_id, fees_storage_item)?;
        return Ok(ProtocolFeesResponse { fees: vec![fee] });
    }

    let fees = fees_storage_item.load(deps.storage)?;
    Ok(ProtocolFeesResponse { fees })
}
