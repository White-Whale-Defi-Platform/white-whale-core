use std::ops::Mul;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    ensure, Addr, Coin, Decimal, Decimal256, Deps, DepsMut, Env, StdError, StdResult, Storage,
    Uint128, Uint256,
};

use white_whale_std::fee::PoolFee;
use white_whale_std::pool_manager::{PoolInfo, PoolType, SimulationResponse};
use white_whale_std::pool_network::asset::{Asset, AssetInfo};

use crate::error::ContractError;
use crate::math::Decimal256Helper;

/// The amount of iterations to perform when calculating the Newton-Raphson approximation.
const NEWTON_ITERATIONS: u64 = 32;

// todo isn't this for the 3pool? shouldn't it be 3
// the number of assets in the pool
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
    Err(ContractError::ConvergeError)
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
                return y.try_into().map_err(|_| ContractError::SwapOverflowError);
            }
        } else if y < previous_y && previous_y.checked_sub(y)? <= Uint256::one() {
            return y.try_into().map_err(|_| ContractError::SwapOverflowError);
        }
    }

    Err(ContractError::ConvergeError)
}

/// computes a swap
pub fn compute_swap(
    offer_pool: Uint128,
    ask_pool: Uint128,
    offer_amount: Uint128,
    pool_fees: PoolFee,
    swap_type: &PoolType,
    offer_precision: u8,
    ask_precision: u8,
) -> Result<SwapComputation, ContractError> {
    let offer_pool: Uint256 = offer_pool.into();
    let ask_pool: Uint256 = ask_pool.into();
    let offer_amount: Uint256 = offer_amount.into();

    match swap_type {
        PoolType::ConstantProduct => {
            // offer => ask
            // ask_amount = (ask_pool * offer_amount / (offer_pool + offer_amount)) - swap_fee - protocol_fee - burn_fee
            let return_amount: Uint256 = Uint256::one()
                * Decimal256::from_ratio(ask_pool.mul(offer_amount), offer_pool + offer_amount);

            // calculate spread, swap and protocol fees
            let exchange_rate = Decimal256::from_ratio(ask_pool, offer_pool);
            let spread_amount: Uint256 = (offer_amount * exchange_rate) - return_amount;

            let fees_computation = compute_fees(pool_fees, return_amount)?;

            Ok(get_swap_computation(
                return_amount,
                spread_amount,
                fees_computation,
            )?)
        }
        PoolType::StableSwap { amp } => {
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

            let fees_computation = compute_fees(pool_fees, return_amount)?;

            Ok(get_swap_computation(
                return_amount,
                spread_amount,
                fees_computation,
            )?)
        }
    }
}

/// Computes the pool fees for a given (return) amount
fn compute_fees(pool_fees: PoolFee, amount: Uint256) -> Result<FeesComputation, ContractError> {
    let swap_fee_amount: Uint256 = pool_fees.swap_fee.compute(amount);
    let protocol_fee_amount: Uint256 = pool_fees.protocol_fee.compute(amount);
    let burn_fee_amount: Uint256 = pool_fees.burn_fee.compute(amount);

    let extra_fees_amount: Uint256 = if !pool_fees.extra_fees.is_empty() {
        let mut extra_fees_amount: Uint256 = Uint256::zero();

        for extra_fee in pool_fees.extra_fees {
            extra_fees_amount = extra_fees_amount.checked_add(extra_fee.compute(amount))?;
        }

        extra_fees_amount
    } else {
        Uint256::zero()
    };

    #[cfg(not(feature = "osmosis"))]
    {
        Ok(FeesComputation {
            swap_fee_amount,
            protocol_fee_amount,
            burn_fee_amount,
            extra_fees_amount,
        })
    }

    #[cfg(feature = "osmosis")]
    {
        let osmosis_fee_amount: Uint256 = pool_fees.osmosis_fee.compute(amount);

        Ok(FeesComputation {
            swap_fee_amount,
            protocol_fee_amount,
            burn_fee_amount,
            extra_fees_amount,
            osmosis_fee_amount,
        })
    }
}

/// Builds the swap computation struct, subtracting the fees from the return amount.
fn get_swap_computation(
    return_amount: Uint256,
    spread_amount: Uint256,
    fees_computation: FeesComputation,
) -> Result<SwapComputation, ContractError> {
    #[cfg(not(feature = "osmosis"))]
    {
        let return_amount = return_amount
            .checked_sub(fees_computation.swap_fee_amount)?
            .checked_sub(fees_computation.protocol_fee_amount)?
            .checked_sub(fees_computation.burn_fee_amount)?
            .checked_sub(fees_computation.extra_fees_amount)?;

        Ok(SwapComputation {
            return_amount: return_amount
                .try_into()
                .map_err(|_| ContractError::SwapOverflowError)?,
            spread_amount: spread_amount
                .try_into()
                .map_err(|_| ContractError::SwapOverflowError)?,
            swap_fee_amount: fees_computation
                .swap_fee_amount
                .try_into()
                .map_err(|_| ContractError::SwapOverflowError)?,
            protocol_fee_amount: fees_computation
                .protocol_fee_amount
                .try_into()
                .map_err(|_| ContractError::SwapOverflowError)?,
            burn_fee_amount: fees_computation
                .burn_fee_amount
                .try_into()
                .map_err(|_| ContractError::SwapOverflowError)?,
            extra_fees_amount: fees_computation
                .extra_fees_amount
                .try_into()
                .map_err(|_| ContractError::SwapOverflowError)?,
        })
    }

    #[cfg(feature = "osmosis")]
    {
        let return_amount = return_amount
            .checked_sub(fees_computation.swap_fee_amount)?
            .checked_sub(fees_computation.protocol_fee_amount)?
            .checked_sub(fees_computation.burn_fee_amount)?
            .checked_sub(fees_computation.extra_fees_amount)?
            .checked_sub(fees_computation.osmosis_fee_amount)?;

        Ok(SwapComputation {
            return_amount: return_amount
                .try_into()
                .map_err(|_| ContractError::SwapOverflowError)?,
            spread_amount: spread_amount
                .try_into()
                .map_err(|_| ContractError::SwapOverflowError)?,
            swap_fee_amount: fees_computation
                .swap_fee_amount
                .try_into()
                .map_err(|_| ContractError::SwapOverflowError)?,
            protocol_fee_amount: fees_computation
                .protocol_fee_amount
                .try_into()
                .map_err(|_| ContractError::SwapOverflowError)?,
            burn_fee_amount: fees_computation
                .burn_fee_amount
                .try_into()
                .map_err(|_| ContractError::SwapOverflowError)?,
            extra_fees_amount: fees_computation
                .extra_fees_amount
                .try_into()
                .map_err(|_| ContractError::SwapOverflowError)?,
            osmosis_fee_amount: fees_computation
                .osmosis_fee_amount
                .try_into()
                .map_err(|_| ContractError::SwapOverflowError)?,
        })
    }
}

/// Represents the swap computation values
#[cw_serde]
pub struct FeesComputation {
    pub swap_fee_amount: Uint256,
    pub protocol_fee_amount: Uint256,
    pub burn_fee_amount: Uint256,
    pub extra_fees_amount: Uint256,
    #[cfg(feature = "osmosis")]
    pub osmosis_fee_amount: Uint256,
}

/// Represents the swap computation values
#[cw_serde]
pub struct SwapComputation {
    pub return_amount: Uint128,
    pub spread_amount: Uint128,
    pub swap_fee_amount: Uint128,
    pub protocol_fee_amount: Uint128,
    pub burn_fee_amount: Uint128,
    pub extra_fees_amount: Uint128,
    #[cfg(feature = "osmosis")]
    pub osmosis_fee_amount: Uint128,
}

impl SwapComputation {
    /// Converts the SwapComputation struct to a SimulationResponse struct
    pub fn to_simulation_response(&self) -> SimulationResponse {
        #[cfg(not(feature = "osmosis"))]
        {
            SimulationResponse {
                return_amount: self.return_amount,
                spread_amount: self.spread_amount,
                swap_fee_amount: self.swap_fee_amount,
                protocol_fee_amount: self.protocol_fee_amount,
                burn_fee_amount: self.burn_fee_amount,
                extra_fees_amount: self.extra_fees_amount,
            }
        }

        #[cfg(feature = "osmosis")]
        {
            SimulationResponse {
                return_amount: self.return_amount,
                spread_amount: self.spread_amount,
                swap_fee_amount: self.swap_fee_amount,
                protocol_fee_amount: self.protocol_fee_amount,
                burn_fee_amount: self.burn_fee_amount,
                osmosis_fee_amount: self.osmosis_fee_amount,
                extra_fees_amount: self.extra_fees_amount,
            }
        }
    }
}

pub fn compute_offer_amount(
    offer_asset_in_pool: Uint128,
    ask_asset_in_pool: Uint128,
    ask_amount: Uint128,
    pool_fees: PoolFee,
) -> StdResult<OfferAmountComputation> {
    let offer_asset_in_pool: Uint256 = offer_asset_in_pool.into();
    let ask_asset_in_pool: Uint256 = ask_asset_in_pool.into();
    let ask_amount: Uint256 = ask_amount.into();

    // ask => offer
    // offer_amount = cp / (ask_pool - ask_amount / (1 - fees)) - offer_pool
    let fees = {
        let base_fees = pool_fees
            .swap_fee
            .to_decimal_256()
            .checked_add(pool_fees.protocol_fee.to_decimal_256())?
            .checked_add(pool_fees.burn_fee.to_decimal_256())?;

        #[cfg(feature = "osmosis")]
        {
            base_fees.checked_add(pool_fees.osmosis_fee.to_decimal_256())?
        }

        #[cfg(not(feature = "osmosis"))]
        {
            base_fees
        }
    };

    let one_minus_commission = Decimal256::one() - fees;
    let inv_one_minus_commission = Decimal256::one() / one_minus_commission;

    let cp: Uint256 = offer_asset_in_pool * ask_asset_in_pool;
    let offer_amount: Uint256 = Uint256::one()
        .multiply_ratio(
            cp,
            ask_asset_in_pool.checked_sub(ask_amount * inv_one_minus_commission)?,
        )
        .checked_sub(offer_asset_in_pool)?;

    let before_commission_deduction: Uint256 = ask_amount * inv_one_minus_commission;
    let before_spread_deduction: Uint256 =
        offer_amount * Decimal256::from_ratio(ask_asset_in_pool, offer_asset_in_pool);

    let spread_amount = before_spread_deduction.saturating_sub(before_commission_deduction);

    let swap_fee_amount: Uint256 = pool_fees.swap_fee.compute(before_commission_deduction);
    let protocol_fee_amount: Uint256 = pool_fees.protocol_fee.compute(before_commission_deduction);
    let burn_fee_amount: Uint256 = pool_fees.burn_fee.compute(before_commission_deduction);

    #[cfg(not(feature = "osmosis"))]
    {
        Ok(OfferAmountComputation {
            offer_amount: offer_amount.try_into()?,
            spread_amount: spread_amount.try_into()?,
            swap_fee_amount: swap_fee_amount.try_into()?,
            protocol_fee_amount: protocol_fee_amount.try_into()?,
            burn_fee_amount: burn_fee_amount.try_into()?,
        })
    }

    #[cfg(feature = "osmosis")]
    {
        let osmosis_fee_amount: Uint256 =
            pool_fees.osmosis_fee.compute(before_commission_deduction);

        Ok(OfferAmountComputation {
            offer_amount: offer_amount.try_into()?,
            spread_amount: spread_amount.try_into()?,
            swap_fee_amount: swap_fee_amount.try_into()?,
            protocol_fee_amount: protocol_fee_amount.try_into()?,
            burn_fee_amount: burn_fee_amount.try_into()?,
            osmosis_fee_amount: osmosis_fee_amount.try_into()?,
        })
    }
}

/// Represents the offer amount computation values
#[cw_serde]
pub struct OfferAmountComputation {
    pub offer_amount: Uint128,
    pub spread_amount: Uint128,
    pub swap_fee_amount: Uint128,
    pub protocol_fee_amount: Uint128,
    pub burn_fee_amount: Uint128,
    #[cfg(feature = "osmosis")]
    pub osmosis_fee_amount: Uint128,
}

pub fn assert_slippage_tolerance(
    slippage_tolerance: &Option<Decimal>,
    deposits: &[Uint128; 2],
    pools: &[Coin; 2],
    pool_type: PoolType,
    amount: Uint128,
    pool_token_supply: Uint128,
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
        match pool_type {
            PoolType::StableSwap { .. } => {
                let pools_total = pools[0].checked_add(pools[1])?;
                let deposits_total = deposits[0].checked_add(deposits[1])?;

                let pool_ratio = Decimal256::from_ratio(pools_total, pool_token_supply);
                let deposit_ratio = Decimal256::from_ratio(deposits_total, amount);

                // the slippage tolerance for the stableswap can't use a simple ratio for calculating
                // slippage when adding liquidity. Due to the math behind the stableswap, the amp factor
                // needs to be in as well, much like when swaps are done
                if pool_ratio * one_minus_slippage_tolerance > deposit_ratio {
                    return Err(ContractError::MaxSlippageAssertion);
                }
            }
            PoolType::ConstantProduct => {
                if Decimal256::from_ratio(deposits[0], deposits[1]) * one_minus_slippage_tolerance
                    > Decimal256::from_ratio(pools[0], pools[1])
                    || Decimal256::from_ratio(deposits[1], deposits[0])
                        * one_minus_slippage_tolerance
                        > Decimal256::from_ratio(pools[1], pools[0])
                {
                    return Err(ContractError::MaxSlippageAssertion);
                }
            }
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
    pool_key: &Vec<u8>,
    fee_storage_item: cw_storage_plus::Map<'static, &'static [u8], Vec<Asset>>,
) -> StdResult<()> {
    fee_storage_item.save(
        storage,
        pool_key,
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

/// This function compares the address of the message sender with the contract admin
/// address. This provides a convenient way to verify if the sender
/// is the admin in a single line.
pub fn assert_admin(deps: Deps, env: &Env, sender: &Addr) -> Result<(), ContractError> {
    let contract_info = deps
        .querier
        .query_wasm_contract_info(env.contract.address.clone())?;
    if let Some(admin) = contract_info.admin {
        if sender != deps.api.addr_validate(admin.as_str())? {
            return Err(ContractError::Unauthorized);
        }
    }
    Ok(())
}

/// Validates the amounts after a single side liquidity provision swap are correct.
pub fn validate_asset_balance(
    deps: &DepsMut,
    env: &Env,
    expected_balance: &Coin,
) -> Result<(), ContractError> {
    let new_asset_balance = deps
        .querier
        .query_balance(&env.contract.address, expected_balance.denom.to_owned())?;

    ensure!(
        expected_balance == &new_asset_balance,
        ContractError::InvalidSingleSideLiquidityProvisionSwap {
            expected: expected_balance.amount,
            actual: new_asset_balance.amount
        }
    );

    Ok(())
}

/// Aggregates the fees from a simulation response that go out of the contract, i.e. protocol fee, burn fee
/// and osmosis fee, if applicable. Doesn't know about the denom, just the amount.
pub fn aggregate_outgoing_fees(
    simulation_response: &SimulationResponse,
) -> Result<Uint128, ContractError> {
    let fees = {
        #[cfg(not(feature = "osmosis"))]
        {
            simulation_response
                .protocol_fee_amount
                .checked_add(simulation_response.burn_fee_amount)?
        }

        #[cfg(feature = "osmosis")]
        {
            simulation_response
                .protocol_fee_amount
                .checked_add(simulation_response.burn_fee_amount)?
                .checked_add(simulation_response.osmosis_fee_amount)?
        }
    };

    Ok(fees)
}

/// Gets the offer and ask asset indexes in a pool, together with their decimals.
pub fn get_asset_indexes_in_pool(
    pool_info: &PoolInfo,
    offer_asset_denom: String,
    ask_asset_denom: String,
) -> Result<(Coin, Coin, usize, usize, u8, u8), ContractError> {
    // Find the index of the offer and ask asset in the pools
    let offer_index = pool_info
        .assets
        .iter()
        .position(|pool| offer_asset_denom == pool.denom)
        .ok_or(ContractError::AssetMismatch)?;
    let ask_index = pool_info
        .assets
        .iter()
        .position(|pool| ask_asset_denom == pool.denom)
        .ok_or(ContractError::AssetMismatch)?;

    // make sure it's not the same asset
    ensure!(offer_index != ask_index, ContractError::AssetMismatch);

    let decimals = &pool_info.asset_decimals;

    let offer_asset_in_pool = pool_info.assets[offer_index].clone();
    let ask_asset_in_pool = pool_info.assets[ask_index].clone();
    let offer_decimal = decimals[offer_index];
    let ask_decimal = decimals[ask_index];

    Ok((
        offer_asset_in_pool,
        ask_asset_in_pool,
        offer_index,
        ask_index,
        offer_decimal,
        ask_decimal,
    ))
}
