use std::cmp::Ordering;

use cosmwasm_std::{coin, ensure, Coin, Decimal256, Deps, Fraction, Order, StdResult, Uint128, Uint256};

use white_whale_std::pool_manager::{
    AssetDecimalsResponse, Config, PoolInfoResponse, PoolType, ReverseSimulationResponse,
    SimulateSwapOperationsResponse, SimulationResponse, SwapOperation, SwapRoute,
    SwapRouteCreatorResponse, SwapRouteResponse, SwapRoutesResponse,
};

use crate::helpers::get_asset_indexes_in_pool;
use crate::state::{CONFIG, POOLS};
use crate::{
    helpers::{self, calculate_stableswap_y, StableSwapDirection},
    state::get_pool_by_identifier,
    ContractError,
};
use crate::{math::Decimal256Helper, state::SWAP_ROUTES};

/// Query the config of the contract.
pub fn query_config(deps: Deps) -> Result<Config, ContractError> {
    Ok(CONFIG.load(deps.storage)?)
}

/// Query the native asset decimals
pub fn query_asset_decimals(
    deps: Deps,
    pool_identifier: String,
    denom: String,
) -> Result<AssetDecimalsResponse, ContractError> {
    let pool_info = get_pool_by_identifier(&deps, &pool_identifier)?;
    let decimal_index = pool_info
        .asset_denoms
        .iter()
        .position(|d| d.clone() == denom)
        .ok_or(ContractError::AssetMismatch)?;

    Ok(AssetDecimalsResponse {
        pool_identifier,
        denom,
        decimals: pool_info.asset_decimals[decimal_index],
    })
}

// Simulate a swap with the provided asset to determine the amount of the other asset that would be received
pub fn query_simulation(
    deps: Deps,
    offer_asset: Coin,
    ask_asset_denom: String,
    pool_identifier: String,
) -> Result<SimulationResponse, ContractError> {
    let pool_info = get_pool_by_identifier(&deps, &pool_identifier)?;

    let (offer_asset_in_pool, ask_asset_in_pool, _, _, offer_decimal, ask_decimal) =
        get_asset_indexes_in_pool(&pool_info, offer_asset.denom, ask_asset_denom)?;

    let swap_computation = helpers::compute_swap(
        Uint256::from(pool_info.assets.len() as u128),
        offer_asset_in_pool.amount,
        ask_asset_in_pool.amount,
        offer_asset.amount,
        pool_info.pool_fees,
        &pool_info.pool_type,
        offer_decimal,
        ask_decimal,
    )?;

    #[cfg(not(feature = "osmosis"))]
    {
        Ok(SimulationResponse {
            return_amount: swap_computation.return_amount,
            spread_amount: swap_computation.spread_amount,
            swap_fee_amount: swap_computation.swap_fee_amount,
            protocol_fee_amount: swap_computation.protocol_fee_amount,
            burn_fee_amount: swap_computation.burn_fee_amount,
            extra_fees_amount: swap_computation.extra_fees_amount,
        })
    }

    #[cfg(feature = "osmosis")]
    {
        Ok(SimulationResponse {
            return_amount: swap_computation.return_amount,
            spread_amount: swap_computation.spread_amount,
            swap_fee_amount: swap_computation.swap_fee_amount,
            protocol_fee_amount: swap_computation.protocol_fee_amount,
            burn_fee_amount: swap_computation.burn_fee_amount,
            extra_fees_amount: swap_computation.extra_fees_amount,
            osmosis_fee_amount: swap_computation.osmosis_fee_amount,
        })
    }
}

/// Queries a swap reverse simulation. Used to derive the number of source tokens returned for
/// the number of target tokens.
pub fn query_reverse_simulation(
    deps: Deps,
    ask_asset: Coin,
    offer_asset_denom: String,
    pool_identifier: String,
) -> Result<ReverseSimulationResponse, ContractError> {
    let pool_info = get_pool_by_identifier(&deps, &pool_identifier)?;

    let (offer_asset_in_pool, ask_asset_in_pool, _, _, offer_decimal, ask_decimal) =
        get_asset_indexes_in_pool(&pool_info, offer_asset_denom, ask_asset.denom)?;

    let pool_fees = pool_info.pool_fees;

    //todo clean this up
    match pool_info.pool_type {
        PoolType::ConstantProduct => {
            let offer_amount_computation = helpers::compute_offer_amount(
                offer_asset_in_pool.amount,
                ask_asset_in_pool.amount,
                ask_asset.amount,
                pool_fees,
            )?;

            #[cfg(not(feature = "osmosis"))]
            {
                Ok(ReverseSimulationResponse {
                    offer_amount: offer_amount_computation.offer_amount,
                    spread_amount: offer_amount_computation.spread_amount,
                    swap_fee_amount: offer_amount_computation.swap_fee_amount,
                    protocol_fee_amount: offer_amount_computation.protocol_fee_amount,
                    burn_fee_amount: offer_amount_computation.burn_fee_amount,
                })
            }

            #[cfg(feature = "osmosis")]
            {
                Ok(ReverseSimulationResponse {
                    offer_amount: offer_amount_computation.offer_amount,
                    spread_amount: offer_amount_computation.spread_amount,
                    swap_fee_amount: offer_amount_computation.swap_fee_amount,
                    protocol_fee_amount: offer_amount_computation.protocol_fee_amount,
                    burn_fee_amount: offer_amount_computation.burn_fee_amount,
                    osmosis_fee_amount: offer_amount_computation.osmosis_fee_amount,
                })
            }
        }
        PoolType::StableSwap { amp } => {
            let offer_pool =
                Decimal256::decimal_with_precision(offer_asset_in_pool.amount, offer_decimal)?;
            let ask_pool =
                Decimal256::decimal_with_precision(ask_asset_in_pool.amount, ask_decimal)?;

            let before_fees = (Decimal256::one()
                .checked_sub(pool_fees.protocol_fee.to_decimal_256())?
                .checked_sub(pool_fees.swap_fee.to_decimal_256())?
                .checked_sub(pool_fees.burn_fee.to_decimal_256())?)
            .inv()
            .unwrap_or_else(Decimal256::one)
            .checked_mul(Decimal256::decimal_with_precision(
                ask_asset.amount,
                ask_decimal,
            )?)?;

            let before_fees_offer = before_fees.to_uint256_with_precision(offer_decimal.into())?;
            let before_fees_ask = before_fees.to_uint256_with_precision(ask_decimal.into())?;

            let max_precision = offer_decimal.max(ask_decimal);

            let new_offer_pool_amount = calculate_stableswap_y(
                Uint256::from(pool_info.assets.len() as u128),
                offer_pool,
                ask_pool,
                before_fees,
                &amp,
                max_precision,
                StableSwapDirection::ReverseSimulate,
            )?;

            let offer_amount = new_offer_pool_amount.checked_sub(Uint128::try_from(
                offer_pool.to_uint256_with_precision(u32::from(max_precision))?,
            )?)?;

            // convert into the original offer precision
            let offer_amount = match max_precision.cmp(&offer_decimal) {
                Ordering::Equal => offer_amount,
                // note that Less should never happen (as max_precision = max(offer_decimal, ask_decimal))
                Ordering::Less => offer_amount.checked_mul(Uint128::new(
                    10u128.pow((offer_decimal - max_precision).into()),
                ))?,
                Ordering::Greater => offer_amount.checked_div(Uint128::new(
                    10u128.pow((max_precision - offer_decimal).into()),
                ))?,
            };

            let spread_amount = offer_amount.saturating_sub(Uint128::try_from(before_fees_offer)?);
            let swap_fee_amount = pool_fees.swap_fee.compute(before_fees_ask);
            let protocol_fee_amount = pool_fees.protocol_fee.compute(before_fees_ask);
            let burn_fee_amount = pool_fees.burn_fee.compute(before_fees_ask);

            #[cfg(not(feature = "osmosis"))]
            {
                Ok(ReverseSimulationResponse {
                    offer_amount,
                    spread_amount,
                    swap_fee_amount: swap_fee_amount.try_into()?,
                    protocol_fee_amount: protocol_fee_amount.try_into()?,
                    burn_fee_amount: burn_fee_amount.try_into()?,
                })
            }

            #[cfg(feature = "osmosis")]
            {
                let osmosis_fee_amount = pool_fees.osmosis_fee.compute(before_fees_ask);

                Ok(ReverseSimulationResponse {
                    offer_amount,
                    spread_amount,
                    swap_fee_amount: swap_fee_amount.try_into()?,
                    protocol_fee_amount: protocol_fee_amount.try_into()?,
                    burn_fee_amount: burn_fee_amount.try_into()?,
                    osmosis_fee_amount: osmosis_fee_amount.try_into()?,
                })
            }
        }
    }
}

// Router related queries, swap routes and SwapOperations
// get_swap_routes which only takes deps: Deps as input
// the function will read from SWAP_ROUTES and return all swap routes in a vec
pub fn get_swap_routes(deps: Deps) -> Result<SwapRoutesResponse, ContractError> {
    let swap_routes: Vec<SwapRoute> = SWAP_ROUTES
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let swap_info = item?;
            // Destructure key into (offer_asset, ask_asset)
            let (offer_asset_denom, ask_asset_denom) = swap_info.0;
            // Destructure value into vec of SwapOperation
            let swap_operations = swap_info.1;

            Ok(SwapRoute {
                offer_asset_denom,
                ask_asset_denom,
                swap_operations: swap_operations.swap_operations,
            })
        })
        .collect::<StdResult<Vec<SwapRoute>>>()?;

    Ok(SwapRoutesResponse { swap_routes })
}

pub fn get_swap_route(
    deps: Deps,
    offer_asset_denom: String,
    ask_asset_denom: String,
) -> Result<SwapRouteResponse, ContractError> {
    let swap_route_key = SWAP_ROUTES.key((&offer_asset_denom, &ask_asset_denom));
    let swap_operations =
        swap_route_key
            .load(deps.storage)
            .map_err(|_| ContractError::NoSwapRouteForAssets {
                offer_asset: offer_asset_denom.clone(),
                ask_asset: ask_asset_denom.clone(),
            })?;
    Ok(SwapRouteResponse {
        swap_route: SwapRoute {
            offer_asset_denom,
            ask_asset_denom,
            swap_operations: swap_operations.swap_operations,
        },
    })
}

pub fn get_swap_route_creator(
    deps: Deps,
    offer_asset_denom: String,
    ask_asset_denom: String,
) -> Result<SwapRouteCreatorResponse, ContractError> {
    let swap_route_key = SWAP_ROUTES.key((&offer_asset_denom, &ask_asset_denom));

    let swap_operations =
        swap_route_key
            .load(deps.storage)
            .map_err(|_| ContractError::NoSwapRouteForAssets {
                offer_asset: offer_asset_denom.clone(),
                ask_asset: ask_asset_denom.clone(),
            })?;
    Ok(SwapRouteCreatorResponse {
        creator: swap_operations.creator,
    })
}

/// Gets the pool info for a given pool identifier. Returns a [PoolInfoResponse].
pub fn get_pool(deps: Deps, pool_identifier: String) -> Result<PoolInfoResponse, ContractError> {
    let pool = POOLS.load(deps.storage, &pool_identifier)?;
    let total_share = deps.querier.query_supply(pool.lp_denom)?;

    Ok(PoolInfoResponse {
        pool_info: POOLS.load(deps.storage, &pool_identifier)?,
        total_share,
    })
}

/// This function iterates over the swap operations, simulates each swap
/// to get the final amount after all the swaps.
pub fn simulate_swap_operations(
    deps: Deps,
    offer_amount: Uint128,
    operations: Vec<SwapOperation>,
) -> Result<SimulateSwapOperationsResponse, ContractError> {
    let operations_len = operations.len();
    ensure!(operations_len > 0, ContractError::NoSwapOperationsProvided);

    let mut amount = offer_amount;

    for operation in operations.into_iter() {
        match operation {
            SwapOperation::WhaleSwap {
                token_in_denom,
                token_out_denom,
                pool_identifier,
            } => {
                let res = query_simulation(
                    deps,
                    coin(amount.u128(), token_in_denom),
                    token_out_denom,
                    pool_identifier,
                )?;
                amount = res.return_amount;
            }
        }
    }

    Ok(SimulateSwapOperationsResponse { amount })
}

/// This function iterates over the swap operations in the reverse order,
/// simulates each swap to get the final amount after all the swaps.
pub fn reverse_simulate_swap_operations(
    deps: Deps,
    ask_amount: Uint128,
    operations: Vec<SwapOperation>,
) -> Result<SimulateSwapOperationsResponse, ContractError> {
    let operations_len = operations.len();
    if operations_len == 0 {
        return Err(ContractError::NoSwapOperationsProvided);
    }

    let mut amount = ask_amount;

    for operation in operations.into_iter().rev() {
        match operation {
            SwapOperation::WhaleSwap {
                token_in_denom,
                token_out_denom,
                pool_identifier,
            } => {
                let res = query_simulation(
                    deps,
                    coin(amount.u128(), token_out_denom),
                    token_in_denom,
                    pool_identifier,
                )?;
                amount = res.return_amount;
            }
        }
    }

    Ok(SimulateSwapOperationsResponse { amount })
}
