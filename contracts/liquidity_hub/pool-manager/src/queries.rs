use std::cmp::Ordering;

use cosmwasm_std::{coin, Coin, Decimal256, Deps, Env, Fraction, Order, StdResult, Uint128};

use white_whale_std::pool_manager::{
    AssetDecimalsResponse, Config, PairInfoResponse, ReverseSimulationResponse,
    SimulateSwapOperationsResponse, SimulationResponse, SwapOperation, SwapRoute,
    SwapRouteCreatorResponse, SwapRouteResponse, SwapRoutesResponse,
};
use white_whale_std::pool_network::asset::PairType;

use crate::state::{CONFIG, PAIRS};
use crate::{
    helpers::{self, calculate_stableswap_y, StableSwapDirection},
    state::get_pair_by_identifier,
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
    pair_identifier: String,
    denom: String,
) -> Result<AssetDecimalsResponse, ContractError> {
    let pair_info = get_pair_by_identifier(&deps, &pair_identifier)?;
    let decimal_index = pair_info
        .asset_denoms
        .iter()
        .position(|d| d.clone() == denom)
        .ok_or(ContractError::AssetMismatch)?;

    Ok(AssetDecimalsResponse {
        pair_identifier,
        denom,
        decimals: pair_info.asset_decimals[decimal_index],
    })
}

// Simulate a swap with the provided asset to determine the amount of the other asset that would be received
pub fn query_simulation(
    deps: Deps,
    offer_asset: Coin,
    pair_identifier: String,
) -> Result<SimulationResponse, ContractError> {
    let pair_info = get_pair_by_identifier(&deps, &pair_identifier)?;
    let pools = pair_info.assets.clone();

    // determine what's the offer and ask pool based on the offer_asset
    let offer_pool: Coin;
    let ask_pool: Coin;
    let offer_decimal: u8;
    let ask_decimal: u8;
    let decimals = pair_info.asset_decimals.clone();
    // We now have the pools and pair info; we can now calculate the swap
    // Verify the pool
    if offer_asset.denom == pools[0].denom {
        offer_pool = pools[0].clone();
        ask_pool = pools[1].clone();
        offer_decimal = decimals[0];
        ask_decimal = decimals[1];
    } else if offer_asset.denom == pools[1].denom {
        offer_pool = pools[1].clone();
        ask_pool = pools[0].clone();

        offer_decimal = decimals[1];
        ask_decimal = decimals[0];
    } else {
        return Err(ContractError::AssetMismatch);
    }

    let pool_fees = pair_info.pool_fees;

    let swap_computation = helpers::compute_swap(
        offer_pool.amount,
        ask_pool.amount,
        offer_asset.amount,
        pool_fees,
        &pair_info.pair_type,
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
            osmosis_fee_amount: swap_computation.osmosis_fee_amount,
        })
    }
}

/// Queries a swap reverse simulation. Used to derive the number of source tokens returned for
/// the number of target tokens.
pub fn query_reverse_simulation(
    deps: Deps,
    _env: Env,
    ask_asset: Coin,
    _offer_asset: Coin,
    pair_identifier: String,
) -> Result<ReverseSimulationResponse, ContractError> {
    let pair_info = get_pair_by_identifier(&deps, &pair_identifier)?;
    let pools = pair_info.assets.clone();

    let decimals = pair_info.asset_decimals.clone();
    let offer_pool: Coin = pools[0].clone();
    let offer_decimal = decimals[0];
    let ask_pool: Coin = pools[1].clone();
    let ask_decimal = decimals[1];
    let pool_fees = pair_info.pool_fees;

    match pair_info.pair_type {
        PairType::ConstantProduct => {
            let offer_amount_computation = helpers::compute_offer_amount(
                offer_pool.amount,
                ask_pool.amount,
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
        PairType::StableSwap { amp } => {
            let offer_pool = Decimal256::decimal_with_precision(offer_pool.amount, offer_decimal)?;
            let ask_pool = Decimal256::decimal_with_precision(ask_pool.amount, ask_decimal)?;

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
// the function will read from SWAP_ROUTES and return all swpa routes in a vec
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

/// Gets the pair info for a given pair identifier. Returns a [PairInfoResponse].
pub fn get_pair(deps: Deps, pair_identifier: String) -> Result<PairInfoResponse, ContractError> {
    let pair = PAIRS.load(deps.storage, &pair_identifier)?;
    let total_share = deps.querier.query_supply(pair.lp_denom)?;

    Ok(PairInfoResponse {
        pair_info: PAIRS.load(deps.storage, &pair_identifier)?,
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
    if operations_len == 0 {
        return Err(ContractError::NoSwapOperationsProvided);
    }

    let mut amount = offer_amount;

    for operation in operations.into_iter() {
        match operation {
            SwapOperation::WhaleSwap {
                token_in_denom,
                token_out_denom: _,
                pool_identifier,
            } => {
                let res =
                    query_simulation(deps, coin(amount.u128(), token_in_denom), pool_identifier)?;
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
                token_in_denom: _,
                token_out_denom,
                pool_identifier,
            } => {
                let res =
                    query_simulation(deps, coin(amount.u128(), token_out_denom), pool_identifier)?;
                amount = res.return_amount;
            }
        }
    }

    Ok(SimulateSwapOperationsResponse { amount })
}
