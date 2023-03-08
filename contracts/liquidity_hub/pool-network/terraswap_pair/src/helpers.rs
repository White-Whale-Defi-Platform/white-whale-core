use std::cmp::Ordering;
use std::ops::Mul;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Decimal256, StdError, StdResult, Storage, Uint128, Uint256};
use cw_storage_plus::Item;

use pool_network::asset::{Asset, AssetInfo, PairType};
use pool_network::pair::PoolFee;

use crate::error::ContractError;
use crate::math::Decimal256Helper;

/// The amount of iterations to perform when calculating the Newton-Raphson approximation.
const NEWTON_ITERATIONS: u64 = 32;

// the number of pools in the pair
const N_COINS: Uint256 = Uint256::from_u128(2);

fn calculate_stableswap_d(
    offer_pool: Decimal256,
    ask_pool: Decimal256,
    amp: &u64,
    precision: u8,
) -> Result<Decimal256, ContractError> {
    let n_coins = Decimal256::from_ratio(N_COINS, Uint256::from_u128(1));

    let sum_pools = offer_pool.checked_add(ask_pool)?;
    if sum_pools.is_zero() {
        // there was nothing to swap, return `0`.
        return Ok(Decimal256::zero());
    }

    // ann = amp * n_coins
    let ann = Decimal256::from_ratio(Uint256::from_u128((*amp).into()).checked_mul(N_COINS)?, 1u8);

    // perform Newton-Raphson method
    let mut current_d = sum_pools;
    for _ in 0..NEWTON_ITERATIONS {
        // multiply each pool by the number of coins
        // and multiply together
        let new_d = [offer_pool, ask_pool]
            .into_iter()
            .try_fold::<_, _, Result<_, ContractError>>(current_d, |acc, pool| {
                let mul_pools = pool.checked_mul(n_coins)?;
                acc.checked_multiply_ratio(current_d, mul_pools)
            })?;

        let old_d = current_d;
        // current_d = ((ann * sum_pools + new_d * n_coins) * current_d) / ((ann - 1) * current_d + (n_coins + 1) * new_d)
        current_d = (ann
            .checked_mul(sum_pools)?
            .checked_add(new_d.checked_mul(n_coins)?)?
            .checked_mul(current_d)?)
        .checked_div(
            (ann.checked_sub(Decimal256::one())?
                .checked_mul(current_d)?
                .checked_add(n_coins.checked_add(Decimal256::one())?.checked_mul(new_d)?))?,
        )?;

        if current_d >= old_d {
            if current_d.checked_sub(old_d)? <= Decimal256::decimal_with_precision(1u8, precision)?
            {
                // success
                return Ok(current_d);
            }
        } else if old_d.checked_sub(current_d)?
            <= Decimal256::decimal_with_precision(1u8, precision)?
        {
            // success
            return Ok(current_d);
        }
    }

    // completed iterations
    // but we never approximated correctly
    Err(ContractError::ConvergeError {})
}

/// Determines the direction of `offer_pool` -> `ask_pool`.
///
/// In a `ReverseSimulate`, we subtract the `offer_pool` from `offer_amount` to get the pool sum.
///
/// In a `Simulate`, we add the two.
pub enum StableSwapDirection {
    Simulate,
    ReverseSimulate,
}

/// Calculates the new pool amount given the current pools and swap size.
pub fn calculate_stableswap_y(
    offer_pool: Decimal256,
    ask_pool: Decimal256,
    offer_amount: Decimal256,
    amp: &u64,
    ask_precision: u8,
    direction: StableSwapDirection,
) -> Result<Uint128, ContractError> {
    let ann = Uint256::from_u128((*amp).into()).checked_mul(N_COINS)?;

    let d = calculate_stableswap_d(offer_pool, ask_pool, amp, ask_precision)?
        .to_uint256_with_precision(u32::from(ask_precision))?;

    let pool_sum = match direction {
        StableSwapDirection::Simulate => offer_pool.checked_add(offer_amount)?,
        StableSwapDirection::ReverseSimulate => ask_pool.checked_sub(offer_amount)?,
    }
    .to_uint256_with_precision(u32::from(ask_precision))?;

    let c = d
        .checked_multiply_ratio(d, pool_sum.checked_mul(N_COINS)?)?
        .checked_multiply_ratio(d, ann.checked_mul(N_COINS)?)?;

    let b = pool_sum.checked_add(d.checked_div(ann)?)?;

    // attempt to converge solution using Newton-Raphson method
    let mut y = d;
    for _ in 0..NEWTON_ITERATIONS {
        let previous_y = y;
        // y = (y^2 + c) / (2y + b - d)
        y = y
            .checked_mul(y)?
            .checked_add(c)?
            .checked_div(y.checked_add(y)?.checked_add(b)?.checked_sub(d)?)?;

        if y >= previous_y {
            if y.checked_sub(previous_y)? <= Uint256::one() {
                return y
                    .try_into()
                    .map_err(|_| ContractError::SwapOverflowError {});
            }
        } else if y < previous_y && previous_y.checked_sub(y)? <= Uint256::one() {
            return y
                .try_into()
                .map_err(|_| ContractError::SwapOverflowError {});
        }
    }

    Err(ContractError::ConvergeError {})
}

pub fn compute_swap(
    offer_pool: Uint128,
    ask_pool: Uint128,
    offer_amount: Uint128,
    pool_fees: PoolFee,
    swap_type: &PairType,
    offer_precision: u8,
    ask_precision: u8,
) -> Result<SwapComputation, ContractError> {
    let offer_pool: Uint256 = offer_pool.into();
    let ask_pool: Uint256 = ask_pool.into();
    let offer_amount: Uint256 = offer_amount.into();

    match swap_type {
        PairType::ConstantProduct => {
            // offer => ask
            // ask_amount = (ask_pool * offer_amount / (offer_pool + offer_amount)) - swap_fee - protocol_fee - burn_fee
            let return_amount: Uint256 = Uint256::one()
                * Decimal256::from_ratio(ask_pool.mul(offer_amount), offer_pool + offer_amount);

            // calculate spread, swap and protocol fees
            let exchange_rate = Decimal256::from_ratio(ask_pool, offer_pool);
            let spread_amount: Uint256 = (offer_amount * exchange_rate) - return_amount;
            let swap_fee_amount: Uint256 = pool_fees.swap_fee.compute(return_amount);
            let protocol_fee_amount: Uint256 = pool_fees.protocol_fee.compute(return_amount);
            let burn_fee_amount: Uint256 = pool_fees.burn_fee.compute(return_amount);

            // swap and protocol fee will be absorbed by the pool. Burn fee amount will be burned on a subsequent msg.
            let return_amount: Uint256 =
                return_amount - swap_fee_amount - protocol_fee_amount - burn_fee_amount;

            Ok(SwapComputation {
                return_amount: return_amount
                    .try_into()
                    .map_err(|_| ContractError::SwapOverflowError {})?,
                spread_amount: spread_amount
                    .try_into()
                    .map_err(|_| ContractError::SwapOverflowError {})?,
                swap_fee_amount: swap_fee_amount
                    .try_into()
                    .map_err(|_| ContractError::SwapOverflowError {})?,
                protocol_fee_amount: protocol_fee_amount
                    .try_into()
                    .map_err(|_| ContractError::SwapOverflowError {})?,
                burn_fee_amount: burn_fee_amount
                    .try_into()
                    .map_err(|_| ContractError::SwapOverflowError {})?,
            })
        }
        PairType::StableSwap { amp } => {
            let offer_pool = Decimal256::decimal_with_precision(offer_pool, offer_precision)?;
            let ask_pool = Decimal256::decimal_with_precision(ask_pool, ask_precision)?;
            let offer_amount = Decimal256::decimal_with_precision(offer_amount, offer_precision)?;

            let new_pool = calculate_stableswap_y(
                offer_pool,
                ask_pool,
                offer_amount,
                amp,
                ask_precision,
                StableSwapDirection::Simulate,
            )?;

            let return_amount = ask_pool
                .to_uint256_with_precision(u32::from(ask_precision))?
                .checked_sub(Uint256::from_uint128(new_pool))?;

            // the spread is the loss from 1:1 conversion
            // thus is it the offer_amount - return_amount
            let spread_amount = offer_amount
                .to_uint256_with_precision(u32::from(ask_precision))?
                .saturating_sub(return_amount);

            // subtract fees from return_amount
            let swap_fee_amount: Uint256 = pool_fees.swap_fee.compute(return_amount);
            let protocol_fee_amount: Uint256 = pool_fees.protocol_fee.compute(return_amount);
            let burn_fee_amount: Uint256 = pool_fees.burn_fee.compute(return_amount);

            let return_amount = return_amount
                .checked_sub(swap_fee_amount)?
                .checked_sub(protocol_fee_amount)?
                .checked_sub(burn_fee_amount)?;

            Ok(SwapComputation {
                return_amount: return_amount
                    .try_into()
                    .map_err(|_| ContractError::SwapOverflowError {})?,
                spread_amount: spread_amount
                    .try_into()
                    .map_err(|_| ContractError::SwapOverflowError {})?,
                swap_fee_amount: swap_fee_amount
                    .try_into()
                    .map_err(|_| ContractError::SwapOverflowError {})?,
                protocol_fee_amount: protocol_fee_amount
                    .try_into()
                    .map_err(|_| ContractError::SwapOverflowError {})?,
                burn_fee_amount: burn_fee_amount
                    .try_into()
                    .map_err(|_| ContractError::SwapOverflowError {})?,
            })
        }
    }
}

/// Represents the swap computation values
#[cw_serde]
pub struct SwapComputation {
    pub return_amount: Uint128,
    pub spread_amount: Uint128,
    pub swap_fee_amount: Uint128,
    pub protocol_fee_amount: Uint128,
    pub burn_fee_amount: Uint128,
}

pub fn compute_offer_amount(
    offer_pool: Uint128,
    ask_pool: Uint128,
    ask_amount: Uint128,
    pool_fees: PoolFee,
) -> StdResult<OfferAmountComputation> {
    let offer_pool: Uint256 = offer_pool.into();
    let ask_pool: Uint256 = ask_pool.into();
    let ask_amount: Uint256 = ask_amount.into();

    // ask => offer
    // offer_amount = cp / (ask_pool - ask_amount / (1 - fees)) - offer_pool
    let fees = pool_fees.swap_fee.to_decimal_256()
        + pool_fees.protocol_fee.to_decimal_256()
        + pool_fees.burn_fee.to_decimal_256();
    let one_minus_commission = Decimal256::one() - fees;
    let inv_one_minus_commission = Decimal256::one() / one_minus_commission;

    let cp: Uint256 = offer_pool * ask_pool;
    let offer_amount: Uint256 = Uint256::one()
        .multiply_ratio(cp, ask_pool - ask_amount * inv_one_minus_commission)
        - offer_pool;

    let before_commission_deduction: Uint256 = ask_amount * inv_one_minus_commission;
    let before_spread_deduction: Uint256 =
        offer_amount * Decimal256::from_ratio(ask_pool, offer_pool);

    let spread_amount = if before_spread_deduction > before_commission_deduction {
        before_spread_deduction - before_commission_deduction
    } else {
        Uint256::zero()
    };

    let swap_fee_amount: Uint256 = pool_fees.swap_fee.compute(before_commission_deduction);
    let protocol_fee_amount: Uint256 = pool_fees.protocol_fee.compute(before_commission_deduction);
    let burn_fee_amount: Uint256 = pool_fees.burn_fee.compute(before_commission_deduction);

    Ok(OfferAmountComputation {
        offer_amount: offer_amount.try_into()?,
        spread_amount: spread_amount.try_into()?,
        swap_fee_amount: swap_fee_amount.try_into()?,
        protocol_fee_amount: protocol_fee_amount.try_into()?,
        burn_fee_amount: burn_fee_amount.try_into()?,
    })
}

/// Represents the offer amount computation values
#[cw_serde]
pub struct OfferAmountComputation {
    pub offer_amount: Uint128,
    pub spread_amount: Uint128,
    pub swap_fee_amount: Uint128,
    pub protocol_fee_amount: Uint128,
    pub burn_fee_amount: Uint128,
}

/// If `belief_price` and `max_spread` both are given,
/// we compute new spread else we just use pool network
/// spread to check `max_spread`
pub fn assert_max_spread(
    belief_price: Option<Decimal>,
    max_spread: Option<Decimal>,
    offer_asset: Asset,
    return_asset: Asset,
    spread_amount: Uint128,
    offer_decimal: u8,
    return_decimal: u8,
) -> Result<(), ContractError> {
    let (offer_amount, return_amount, spread_amount): (Uint256, Uint256, Uint256) =
        match offer_decimal.cmp(&return_decimal) {
            Ordering::Greater => {
                let diff_decimal = 10u64.pow((offer_decimal - return_decimal).into());

                (
                    offer_asset.amount.into(),
                    return_asset
                        .amount
                        .checked_mul(Uint128::from(diff_decimal))?
                        .into(),
                    spread_amount
                        .checked_mul(Uint128::from(diff_decimal))?
                        .into(),
                )
            }
            Ordering::Less => {
                let diff_decimal = 10u64.pow((return_decimal - offer_decimal).into());

                (
                    offer_asset
                        .amount
                        .checked_mul(Uint128::from(diff_decimal))?
                        .into(),
                    return_asset.amount.into(),
                    spread_amount.into(),
                )
            }
            Ordering::Equal => (
                offer_asset.amount.into(),
                return_asset.amount.into(),
                spread_amount.into(),
            ),
        };

    if let (Some(max_spread), Some(belief_price)) = (max_spread, belief_price) {
        let belief_price: Decimal256 = belief_price.into();
        let max_spread: Decimal256 = max_spread.into();

        let expected_return = offer_amount * (Decimal256::one() / belief_price);
        let spread_amount = expected_return.saturating_sub(return_amount);

        if return_amount < expected_return
            && Decimal256::from_ratio(spread_amount, expected_return) > max_spread
        {
            return Err(ContractError::MaxSpreadAssertion {});
        }
    } else if let Some(max_spread) = max_spread {
        let max_spread: Decimal256 = max_spread.into();
        if Decimal256::from_ratio(spread_amount, return_amount + spread_amount) > max_spread {
            return Err(ContractError::MaxSpreadAssertion {});
        }
    }

    Ok(())
}

pub fn assert_slippage_tolerance(
    slippage_tolerance: &Option<Decimal>,
    deposits: &[Uint128; 2],
    pools: &[Asset; 2],
) -> Result<(), ContractError> {
    if let Some(slippage_tolerance) = *slippage_tolerance {
        let slippage_tolerance: Decimal256 = slippage_tolerance.into();
        if slippage_tolerance > Decimal256::one() {
            return Err(StdError::generic_err("slippage_tolerance cannot bigger than 1").into());
        }

        let one_minus_slippage_tolerance = Decimal256::one() - slippage_tolerance;
        let deposits: [Uint256; 2] = [deposits[0].into(), deposits[1].into()];
        let pools: [Uint256; 2] = [pools[0].amount.into(), pools[1].amount.into()];

        // Ensure each prices are not dropped as much as slippage tolerance rate
        if Decimal256::from_ratio(deposits[0], deposits[1]) * one_minus_slippage_tolerance
            > Decimal256::from_ratio(pools[0], pools[1])
            || Decimal256::from_ratio(deposits[1], deposits[0]) * one_minus_slippage_tolerance
                > Decimal256::from_ratio(pools[1], pools[0])
        {
            return Err(ContractError::MaxSlippageAssertion {});
        }
    }

    Ok(())
}

/// Gets the protocol fee amount for the given asset_id
pub fn get_protocol_fee_for_asset(
    collected_protocol_fees: Vec<Asset>,
    asset_id: String,
) -> Uint128 {
    let protocol_fee_asset = collected_protocol_fees
        .iter()
        .find(|&protocol_fee_asset| protocol_fee_asset.clone().get_id() == asset_id.clone())
        .cloned();

    // get the protocol fee for the given pool_asset
    if let Some(protocol_fee_asset) = protocol_fee_asset {
        protocol_fee_asset.amount
    } else {
        Uint128::zero()
    }
}

/// Instantiates fees for a given fee_storage_item
pub fn instantiate_fees(
    storage: &mut dyn Storage,
    asset_info_0: AssetInfo,
    asset_info_1: AssetInfo,
    fee_storage_item: Item<Vec<Asset>>,
) -> StdResult<()> {
    fee_storage_item.save(
        storage,
        &vec![
            Asset {
                info: asset_info_0,
                amount: Uint128::zero(),
            },
            Asset {
                info: asset_info_1,
                amount: Uint128::zero(),
            },
        ],
    )
}
