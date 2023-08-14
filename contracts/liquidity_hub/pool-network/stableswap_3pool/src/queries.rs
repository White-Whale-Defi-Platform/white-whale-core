use classic_bindings::TerraQuery;
use cosmwasm_std::{Deps, StdResult};
use cw_storage_plus::Item;

use white_whale::pool_network::asset::{Asset, AssetInfoRaw, TrioInfo, TrioInfoRaw};
use white_whale::pool_network::trio::{
    ConfigResponse, PoolResponse, ProtocolFeesResponse, ReverseSimulationResponse,
    SimulationResponse,
};

use crate::error::ContractError;
use crate::helpers;
use crate::helpers::{get_protocol_fee_for_asset, get_total_share};
use crate::stableswap_math::curve::StableSwap;
use crate::state::{get_fees_for_asset, COLLECTED_PROTOCOL_FEES, CONFIG, TRIO_INFO};

/// Queries the [TrioInfo] of the pool
pub fn query_trio_info(deps: Deps<TerraQuery>) -> Result<TrioInfo, ContractError> {
    let trio_info: TrioInfoRaw = TRIO_INFO.load(deps.storage)?;
    let trio_info = trio_info.to_normal(deps.api)?;

    Ok(trio_info)
}

/// Queries the Pool info, i.e. Assets and total share
pub fn query_pool(deps: Deps<TerraQuery>) -> Result<PoolResponse, ContractError> {
    let trio_info: TrioInfoRaw = TRIO_INFO.load(deps.storage)?;
    let contract_addr = deps.api.addr_humanize(&trio_info.contract_addr)?;

    let collected_protocol_fees = COLLECTED_PROTOCOL_FEES.load(deps.storage)?;
    let assets = trio_info
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

    let liquidity_token = match trio_info.liquidity_token {
        AssetInfoRaw::Token { contract_addr } => {
            deps.api.addr_humanize(&contract_addr)?.to_string()
        }
        AssetInfoRaw::NativeToken { denom } => denom,
    };

    let total_share = get_total_share(&deps, liquidity_token)?;

    let resp = PoolResponse {
        assets,
        total_share,
    };

    Ok(resp)
}

/// Queries a swap simulation. Used to know how much the target asset will be returned for the source token
pub fn query_simulation(
    deps: Deps<TerraQuery>,
    offer_asset: Asset,
    ask_asset: Asset,
    current_block: u64,
) -> Result<SimulationResponse, ContractError> {
    let trio_info: TrioInfoRaw = TRIO_INFO.load(deps.storage)?;

    let contract_addr = deps.api.addr_humanize(&trio_info.contract_addr)?;

    let collected_protocol_fees = COLLECTED_PROTOCOL_FEES.load(deps.storage)?;

    // To calculate pool amounts properly we should subtract the protocol fees from the pool
    let pools = trio_info
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

    let ask_pool: Asset;
    let offer_pool: Asset;
    let unswapped_pool: Asset;

    if ask_asset.info.equal(&pools[0].info) {
        if offer_asset.info.equal(&pools[1].info) {
            ask_pool = pools[0].clone();
            offer_pool = pools[1].clone();
            unswapped_pool = pools[2].clone();
        } else if offer_asset.info.equal(&pools[2].info) {
            ask_pool = pools[0].clone();
            offer_pool = pools[2].clone();
            unswapped_pool = pools[1].clone();
        } else {
            return Err(ContractError::AssetMismatch {});
        }
    } else if ask_asset.info.equal(&pools[1].info) {
        if offer_asset.info.equal(&pools[0].info) {
            ask_pool = pools[1].clone();
            offer_pool = pools[0].clone();
            unswapped_pool = pools[2].clone();
        } else if offer_asset.info.equal(&pools[2].info) {
            ask_pool = pools[1].clone();
            offer_pool = pools[2].clone();
            unswapped_pool = pools[0].clone();
        } else {
            return Err(ContractError::AssetMismatch {});
        }
    } else if ask_asset.info.equal(&pools[2].info) {
        if offer_asset.info.equal(&pools[0].info) {
            ask_pool = pools[2].clone();
            offer_pool = pools[0].clone();
            unswapped_pool = pools[1].clone();
        } else if offer_asset.info.equal(&pools[1].info) {
            ask_pool = pools[2].clone();
            offer_pool = pools[1].clone();
            unswapped_pool = pools[0].clone();
        } else {
            return Err(ContractError::AssetMismatch {});
        }
    } else {
        return Err(ContractError::AssetMismatch {});
    }

    let config = CONFIG.load(deps.storage)?;
    let invariant = StableSwap::new(
        config.initial_amp,
        config.future_amp,
        current_block,
        config.initial_amp_block,
        config.future_amp_block,
    );

    let swap_computation = helpers::compute_swap(
        offer_pool.amount,
        ask_pool.amount,
        unswapped_pool.amount,
        offer_asset.amount,
        config.pool_fees,
        invariant,
    )?;

    Ok(SimulationResponse {
        return_amount: swap_computation.return_amount,
        spread_amount: swap_computation.spread_amount,
        swap_fee_amount: swap_computation.swap_fee_amount,
        protocol_fee_amount: swap_computation.protocol_fee_amount,
        burn_fee_amount: swap_computation.burn_fee_amount,
    })
}

/// Queries a swap reverse simulation. Used to derive the number of source tokens returned for
/// the number of target tokens.
pub fn query_reverse_simulation(
    deps: Deps<TerraQuery>,
    ask_asset: Asset,
    offer_asset: Asset,
    current_block: u64,
) -> Result<ReverseSimulationResponse, ContractError> {
    let trio_info: TrioInfoRaw = TRIO_INFO.load(deps.storage)?;

    let contract_addr = deps.api.addr_humanize(&trio_info.contract_addr)?;

    // To calculate pool amounts properly we should subtract the protocol fees from the pool
    let collected_protocol_fees = COLLECTED_PROTOCOL_FEES.load(deps.storage)?;

    let pools = trio_info
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

    let ask_pool: Asset;
    let offer_pool: Asset;
    let unswapped_pool: Asset;

    if ask_asset.info.equal(&pools[0].info) {
        if offer_asset.info.equal(&pools[1].info) {
            ask_pool = pools[0].clone();
            offer_pool = pools[1].clone();
            unswapped_pool = pools[2].clone();
        } else if offer_asset.info.equal(&pools[2].info) {
            ask_pool = pools[0].clone();
            offer_pool = pools[2].clone();
            unswapped_pool = pools[1].clone();
        } else {
            return Err(ContractError::AssetMismatch {});
        }
    } else if ask_asset.info.equal(&pools[1].info) {
        if offer_asset.info.equal(&pools[0].info) {
            ask_pool = pools[1].clone();
            offer_pool = pools[0].clone();
            unswapped_pool = pools[2].clone();
        } else if offer_asset.info.equal(&pools[2].info) {
            ask_pool = pools[1].clone();
            offer_pool = pools[2].clone();
            unswapped_pool = pools[0].clone();
        } else {
            return Err(ContractError::AssetMismatch {});
        }
    } else if ask_asset.info.equal(&pools[2].info) {
        if offer_asset.info.equal(&pools[0].info) {
            ask_pool = pools[2].clone();
            offer_pool = pools[0].clone();
            unswapped_pool = pools[1].clone();
        } else if offer_asset.info.equal(&pools[1].info) {
            ask_pool = pools[2].clone();
            offer_pool = pools[1].clone();
            unswapped_pool = pools[0].clone();
        } else {
            return Err(ContractError::AssetMismatch {});
        }
    } else {
        return Err(ContractError::AssetMismatch {});
    }

    let config = CONFIG.load(deps.storage)?;
    let invariant = StableSwap::new(
        config.initial_amp,
        config.future_amp,
        current_block,
        config.initial_amp_block,
        config.future_amp_block,
    );

    let offer_amount_computation = helpers::compute_offer_amount(
        offer_pool.amount,
        ask_pool.amount,
        unswapped_pool.amount,
        ask_asset.amount,
        config.pool_fees,
        invariant,
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
pub fn query_config(deps: Deps<TerraQuery>) -> Result<ConfigResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}

/// Queries the fees on the pool for the given fees_storage_item
pub fn query_fees(
    deps: Deps<TerraQuery>,
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
