use std::cmp::Ordering;

use cosmwasm_std::{Decimal256, Deps, DepsMut, Env, Fraction, StdResult, Uint128};
use white_whale::pool_network::{
    asset::{Asset, AssetInfo, AssetInfoRaw, PairType},
    factory::NativeTokenDecimalsResponse,
    pair::{ReverseSimulationResponse, SimulationResponse},
};

use crate::math::Decimal256Helper;
use crate::{
    helpers::{self, calculate_stableswap_y, get_protocol_fee_for_asset, StableSwapDirection},
    state::{
        get_decimals, pair_key, ALLOW_NATIVE_TOKENS, COLLECTABLE_PROTOCOL_FEES, MANAGER_CONFIG,
        PAIRS,
    },
    ContractError,
};

/// Query the native token decimals
pub fn query_native_token_decimal(
    deps: Deps,
    denom: String,
) -> Result<NativeTokenDecimalsResponse, ContractError> {
    let decimals = ALLOW_NATIVE_TOKENS.load(deps.storage, denom.as_bytes())?;

    Ok(NativeTokenDecimalsResponse { decimals })
}

fn get_pair_key_from_assets(
    assets: &[AssetInfo],
    deps: &Deps<'_>,
) -> Result<Vec<u8>, ContractError> {
    let raw_infos: Vec<AssetInfoRaw> = assets
        .iter()
        .map(|asset| asset.to_raw(deps.api))
        .collect::<Result<_, _>>()?;
    let pair_key = pair_key(&raw_infos);
    Ok(pair_key)
}

// TODO: Might be handy for organisation to have a couple of sub-modules here
// Simulation with only stuff for that, swap routes and so on. Otherwise this file might become massive with all the queries

// Simulate a swap with the provided asset to determine the amount of the other asset that would be received
pub fn query_simulation(
    deps: Deps,
    env: Env,
    offer_asset: Asset,
    ask_asset: Asset,
) -> Result<SimulationResponse, ContractError> {
    let assets = [offer_asset.clone(), ask_asset.clone()];
    let asset_infos = [offer_asset.info.clone(), ask_asset.info.clone()];
    let (_assets_vec, pools, pair_info) = match assets {
        // For TWO assets we use the constant product logic
        assets if assets.len() == 2 => {
            let pair_key = get_pair_key_from_assets(&asset_infos, &deps)?;
            let pair_info = PAIRS.load(deps.storage, &pair_key)?;
            let pools: [Asset; 2] = [
                Asset {
                    info: asset_infos[0].clone(),
                    amount: asset_infos[0].query_pool(
                        &deps.querier,
                        deps.api,
                        env.contract.address.clone(),
                    )?,
                },
                Asset {
                    info: asset_infos[1].clone(),
                    amount: asset_infos[1].query_pool(
                        &deps.querier,
                        deps.api,
                        env.contract.address,
                    )?,
                },
            ];

            (assets.to_vec(), pools.to_vec(), pair_info)
        }
        // For both THREE and N we use the same logic; stableswap or eventually conc liquidity
        assets if assets.len() == 3 => {
            let pair_key = get_pair_key_from_assets(&asset_infos, &deps)?;
            let pair_info = PAIRS.load(deps.storage, &pair_key)?;
            // TODO: this is fucked, rework later after constant product working
            let asset_infos = [
                offer_asset.info.clone(),
                ask_asset.info.clone(),
                ask_asset.info.clone(),
            ];
            let assets = [offer_asset.clone(), ask_asset.clone(), ask_asset];

            let pools: [Asset; 3] = [
                Asset {
                    info: asset_infos[0].clone(),
                    amount: asset_infos[0].query_pool(
                        &deps.querier,
                        deps.api,
                        env.contract.address.clone(),
                    )?,
                },
                Asset {
                    info: asset_infos[1].clone(),
                    amount: asset_infos[1].query_pool(
                        &deps.querier,
                        deps.api,
                        env.contract.address.clone(),
                    )?,
                },
                Asset {
                    info: asset_infos[2].clone(),
                    amount: asset_infos[2].query_pool(
                        &deps.querier,
                        deps.api,
                        env.contract.address,
                    )?,
                },
            ];

            (assets.to_vec(), pools.to_vec(), pair_info)
        }
        _ => {
            return Err(ContractError::TooManyAssets {
                assets_provided: assets.len(),
            })
        }
    };
    let offer_pool: Asset;
    let offer_decimal;

    let ask_pool: Asset;
    let ask_decimal;
    let decimals = get_decimals(&pair_info);

    let collected_protocol_fees =
        COLLECTABLE_PROTOCOL_FEES.load(deps.storage, &pair_info.liquidity_token.to_string())?;

    // To calculate pool amounts properly we should subtract the protocol fees from the pool
    pools
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
        offer_decimal = decimals[0];

        ask_pool = pools[1].clone();
        ask_decimal = decimals[1];
    } else if offer_asset.info.equal(&pools[1].info) {
        offer_pool = pools[1].clone();
        offer_decimal = decimals[1];

        ask_pool = pools[0].clone();
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
    env: Env,
    ask_asset: Asset,
    offer_asset: Asset,
) -> Result<ReverseSimulationResponse, ContractError> {
    let assets = [offer_asset.clone(), ask_asset.clone()];
    let asset_infos = [offer_asset.info.clone(), ask_asset.info.clone()];
    let (_assets_vec, pools, pair_info) = match assets {
        // For TWO assets we use the constant product logic
        assets if assets.len() == 2 => {
            let pair_key = get_pair_key_from_assets(&asset_infos, &deps)?;
            let pair_info = PAIRS.load(deps.storage, &pair_key)?;
            let pools: [Asset; 2] = [
                Asset {
                    info: asset_infos[0].clone(),
                    amount: asset_infos[0].query_pool(
                        &deps.querier,
                        deps.api,
                        env.contract.address.clone(),
                    )?,
                },
                Asset {
                    info: asset_infos[1].clone(),
                    amount: asset_infos[1].query_pool(
                        &deps.querier,
                        deps.api,
                        env.contract.address,
                    )?,
                },
            ];

            (assets.to_vec(), pools.to_vec(), pair_info)
        }
        // For both THREE and N we use the same logic; stableswap or eventually conc liquidity
        assets if assets.len() == 3 => {
            let pair_key = get_pair_key_from_assets(&asset_infos, &deps)?;
            let pair_info = PAIRS.load(deps.storage, &pair_key)?;
            // TODO: this is fucked, rework later after constant product working
            let asset_infos = [
                offer_asset.info.clone(),
                ask_asset.info.clone(),
                ask_asset.info.clone(),
            ];
            let assets = [offer_asset.clone(), ask_asset.clone(), ask_asset];

            let pools: [Asset; 3] = [
                Asset {
                    info: asset_infos[0].clone(),
                    amount: asset_infos[0].query_pool(
                        &deps.querier,
                        deps.api,
                        env.contract.address.clone(),
                    )?,
                },
                Asset {
                    info: asset_infos[1].clone(),
                    amount: asset_infos[1].query_pool(
                        &deps.querier,
                        deps.api,
                        env.contract.address.clone(),
                    )?,
                },
                Asset {
                    info: asset_infos[2].clone(),
                    amount: asset_infos[2].query_pool(
                        &deps.querier,
                        deps.api,
                        env.contract.address,
                    )?,
                },
            ];

            (assets.to_vec(), pools.to_vec(), pair_info)
        }
        _ => {
            return Err(ContractError::TooManyAssets {
                assets_provided: assets.len(),
            })
        }
    };
    let offer_pool: Asset;
    let offer_decimal;

    let ask_pool: Asset;
    let ask_decimal;
    let decimals = get_decimals(&pair_info);

    let collected_protocol_fees =
        COLLECTABLE_PROTOCOL_FEES.load(deps.storage, &pair_info.liquidity_token.to_string())?;
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
