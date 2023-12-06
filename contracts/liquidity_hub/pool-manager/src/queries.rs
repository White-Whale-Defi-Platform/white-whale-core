use std::cmp::Ordering;

use cosmwasm_std::{Decimal256, Deps, Env, Fraction, Order, StdResult, Uint128};
use white_whale::pool_manager::{SwapOperation, SwapRouteResponse};
use white_whale::pool_network::{
    asset::{Asset, AssetInfo, PairType},
    factory::NativeTokenDecimalsResponse,
    pair::{ReverseSimulationResponse, SimulationResponse},
    router::SimulateSwapOperationsResponse,
};

use crate::{
    helpers::{self, calculate_stableswap_y, StableSwapDirection},
    state::{
        get_decimals, get_pair_by_identifier, ALLOW_NATIVE_TOKENS,
    },
    ContractError,
};
use crate::{math::Decimal256Helper, state::SWAP_ROUTES};

/// Query the native token decimals
pub fn query_native_token_decimal(
    deps: Deps,
    denom: String,
) -> Result<NativeTokenDecimalsResponse, ContractError> {
    let decimals = ALLOW_NATIVE_TOKENS.load(deps.storage, denom.as_bytes())?;

    Ok(NativeTokenDecimalsResponse { decimals })
}

// Simulate a swap with the provided asset to determine the amount of the other asset that would be received
pub fn query_simulation(
    deps: Deps,
    _env: Env,
    offer_asset: Asset,
    _ask_asset: Asset,
    pair_identifier: String,
) -> Result<SimulationResponse, ContractError> {
    let pair_info = get_pair_by_identifier(&deps, pair_identifier.clone())?;
    let pools = pair_info.assets.clone();
    // determine what's the offer and ask pool based on the offer_asset
    let offer_pool: Asset;
    let ask_pool: Asset;
    let offer_decimal: u8;
    let ask_decimal: u8;
    let decimals = get_decimals(&pair_info);
    // We now have the pools and pair info; we can now calculate the swap
    // Verify the pool
    if offer_asset.info.equal(&pools[0].info) {
        offer_pool = pools[0].clone();
        ask_pool = pools[1].clone();
        offer_decimal = decimals[0];
        ask_decimal = decimals[1];
    } else if offer_asset.info.equal(&pools[1].info) {
        offer_pool = pools[1].clone();
        ask_pool = pools[0].clone();

        offer_decimal = decimals[1];
        ask_decimal = decimals[0];
    } else {
        return Err(ContractError::AssetMismatch {});
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
    deps: Deps,
    _env: Env,
    ask_asset: Asset,
    _offer_asset: Asset,
    pair_identifier: String,
) -> Result<ReverseSimulationResponse, ContractError> {
    let pair_info = get_pair_by_identifier(&deps, pair_identifier.clone())?;
    let pools = pair_info.assets.clone();    
    
    let decimals = get_decimals(&pair_info);
    let offer_pool: Asset = pools[0].clone();
    let offer_decimal = decimals[0];
    let ask_pool: Asset = pools[1].clone();
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

            Ok(ReverseSimulationResponse {
                offer_amount: offer_amount_computation.offer_amount,
                spread_amount: offer_amount_computation.spread_amount,
                swap_fee_amount: offer_amount_computation.swap_fee_amount,
                protocol_fee_amount: offer_amount_computation.protocol_fee_amount,
                burn_fee_amount: offer_amount_computation.burn_fee_amount,
            })
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

            Ok(ReverseSimulationResponse {
                offer_amount,
                spread_amount,
                swap_fee_amount: swap_fee_amount.try_into()?,
                protocol_fee_amount: protocol_fee_amount.try_into()?,
                burn_fee_amount: burn_fee_amount.try_into()?,
            })
        }
    }
}

// Router related queries, swap routes and SwapOperations
// get_swap_routes which only takes deps: Deps as input
// the function will read from SWAP_ROUTES and return all swpa routes in a vec
pub fn get_swap_routes(deps: Deps) -> Result<Vec<SwapRouteResponse>, ContractError> {
    let swap_routes: Vec<SwapRouteResponse> = SWAP_ROUTES
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let swap_info = item?;
            // Destructure key into (offer_asset, ask_asset)
            let (offer_asset, ask_asset) = swap_info.0;
            // Destructure value into vec of SwapOperation
            let swap_route = swap_info.1;

            Ok(SwapRouteResponse {
                offer_asset,
                ask_asset,
                swap_route,
            })
        })
        .collect::<StdResult<Vec<SwapRouteResponse>>>()?;

    Ok(swap_routes)
}

pub fn get_swap_route(
    deps: Deps,
    offer_asset_info: AssetInfo,
    ask_asset_info: AssetInfo,
) -> Result<Vec<SwapOperation>, ContractError> {
    let swap_route_key = SWAP_ROUTES.key((
        offer_asset_info.clone().get_label(&deps)?.as_str(),
        ask_asset_info.clone().get_label(&deps)?.as_str(),
    ));

    swap_route_key
        .load(deps.storage)
        .map_err(|_| ContractError::NoSwapRouteForAssets {
            offer_asset: offer_asset_info.to_string(),
            ask_asset: ask_asset_info.to_string(),
        })
}

// TODO: May need to remove this for a new implementation, router swap operation queries 
// pub fn simulate_swap_operations(
//     deps: Deps,
//     env: Env,
//     offer_amount: Uint128,
//     operations: Vec<SwapOperation>,
// ) -> Result<SimulateSwapOperationsResponse, ContractError> {
//     let operations_len = operations.len();
//     if operations_len == 0 {
//         return Err(ContractError::NoSwapOperationsProvided {});
//     }

//     let mut offer_amount = offer_amount;
//     for operation in operations.into_iter() {
//         match operation {
//             SwapOperation::WhaleSwap {
//                 token_in_info,
//                 token_out_info,
//                 pool_identifier,
//             } => {
//                 let res: SimulationResponse = query_simulation(
//                     deps,
//                     env.clone(),
//                     Asset {
//                         info: token_in_info,
//                         amount: offer_amount,
//                     },
//                     Asset {
//                         info: token_out_info,
//                         amount: Uint128::zero(),
//                     },
//                     pool_identifier,
//                 )?;

//                 offer_amount = res.return_amount;
//             }
//         }
//     }

//     Ok(SimulateSwapOperationsResponse {
//         amount: offer_amount,
//     })
// }

// pub fn reverse_simulate_swap_operations(
//     deps: Deps,
//     env: Env,
//     ask_amount: Uint128,
//     operations: Vec<SwapOperation>,
// ) -> Result<SimulateSwapOperationsResponse, ContractError> {
//     let operations_len = operations.len();
//     if operations_len == 0 {
//         return Err(ContractError::NoSwapOperationsProvided {});
//     }

//     let mut ask_amount = ask_amount;
//     for operation in operations.into_iter().rev() {
//         ask_amount = match operation {
//             SwapOperation::WhaleSwap {
//                 token_in_info: offer_asset_info,
//                 token_out_info: ask_asset_info,
//                 pool_identifier,
//             } => reverse_simulate_return_amount(
//                 deps,
//                 env.clone(),
//                 ask_amount,
//                 offer_asset_info,
//                 ask_asset_info,
//                 pool_identifier,
//             )?,
//         }
//     }

//     Ok(SimulateSwapOperationsResponse { amount: ask_amount })
// }

// pub fn reverse_simulate_return_amount(
//     deps: Deps,
//     env: Env,
//     _ask_amount: Uint128,
//     offer_asset_info: AssetInfo,
//     ask_asset_info: AssetInfo,
//     pool_identifier: String,
// ) -> Result<Uint128, ContractError> {
//     let _pair_info = get_pair_by_identifier(&deps, pool_identifier.clone())?;

//     let res: ReverseSimulationResponse = query_reverse_simulation(
//         deps,
//         env,
//         Asset {
//             info: ask_asset_info,
//             amount: Uint128::zero(),
//         },
//         Asset {
//             info: offer_asset_info,
//             amount: Uint128::zero(),
//         },
//         pool_identifier,
//     )?;

//     Ok(res.offer_amount)
// }
