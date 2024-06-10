use std::ops::Mul;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    coin, ensure, Addr, Coin, Decimal, Decimal256, Deps, DepsMut, Env, StdError, StdResult,
    Storage, Uint128, Uint256,
};

use white_whale_std::fee::PoolFee;
use white_whale_std::pool_manager::{PoolInfo, PoolType, SimulationResponse};
use white_whale_std::pool_network::asset::{Asset, AssetInfo};

use crate::error::ContractError;
use crate::math::Decimal256Helper;

/// The amount of iterations to perform when calculating the Newton-Raphson approximation.
const NEWTON_ITERATIONS: u64 = 32;

/// Encodes all results of swapping from a source token to a destination token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SwapResult {
    /// New amount of source token
    pub new_source_amount: Uint128,
    /// New amount of destination token
    pub new_destination_amount: Uint128,
    /// Amount of destination token swapped
    pub amount_swapped: Uint128,
}

fn calculate_stableswap_d(
    n_coins: Uint256,
    offer_pool: Decimal256,
    ask_pool: Decimal256,
    amp: &u64,
    precision: u8,
) -> Result<Decimal256, ContractError> {
    let n_coins_decimal = Decimal256::from_ratio(n_coins, Uint256::one());

    let sum_pools = offer_pool.checked_add(ask_pool)?;
    if sum_pools.is_zero() {
        // there was nothing to swap, return `0`.
        return Ok(Decimal256::zero());
    }

    // ann = amp * n_coins
    let ann = Decimal256::from_ratio(Uint256::from_u128((*amp).into()).checked_mul(n_coins)?, 1u8);

    // perform Newton-Raphson method
    let mut current_d = sum_pools;
    for _ in 0..NEWTON_ITERATIONS {
        // multiply each pool by the number of coins
        // and multiply together
        let new_d = [offer_pool, ask_pool]
            .into_iter()
            .try_fold::<_, _, Result<_, ContractError>>(current_d, |acc, pool| {
                let mul_pools = pool.checked_mul(n_coins_decimal)?;
                acc.checked_multiply_ratio(current_d, mul_pools)
            })?;

        let old_d = current_d;
        // current_d = ((ann * sum_pools + new_d * n_coins) * current_d) / ((ann - 1) * current_d + (n_coins + 1) * new_d)
        current_d = (ann
            .checked_mul(sum_pools)?
            .checked_add(new_d.checked_mul(n_coins_decimal)?)?
            .checked_mul(current_d)?)
        .checked_div(
            (ann.checked_sub(Decimal256::one())?
                .checked_mul(current_d)?
                .checked_add(
                    n_coins_decimal
                        .checked_add(Decimal256::one())?
                        .checked_mul(new_d)?,
                ))?,
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
    n_coins: Uint256,
    offer_pool: Decimal256,
    ask_pool: Decimal256,
    offer_amount: Decimal256,
    amp: &u64,
    ask_precision: u8,
    direction: StableSwapDirection,
) -> Result<Uint128, ContractError> {
    let ann = Uint256::from_u128((*amp).into()).checked_mul(n_coins)?;

    let d = calculate_stableswap_d(n_coins, offer_pool, ask_pool, amp, ask_precision)?
        .to_uint256_with_precision(u32::from(ask_precision))?;

    let pool_sum = match direction {
        StableSwapDirection::Simulate => offer_pool.checked_add(offer_amount)?,
        StableSwapDirection::ReverseSimulate => ask_pool.checked_sub(offer_amount)?,
    }
    .to_uint256_with_precision(u32::from(ask_precision))?;

    let c = d
        .checked_multiply_ratio(d, pool_sum.checked_mul(n_coins)?)?
        .checked_multiply_ratio(d, ann.checked_mul(n_coins)?)?;

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

#[allow(clippy::too_many_arguments)]
/// computes a swap
#[allow(clippy::too_many_arguments)]
pub fn compute_swap(
    n_coins: Uint256,
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
            let exchange_rate = Decimal256::checked_from_ratio(ask_pool, offer_pool)
                .map_err(|_| ContractError::PoolHasNoAssets)?;
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
                n_coins,
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

// TODO: make this work with n_coins being dynamic

pub fn assert_slippage_tolerance(
    slippage_tolerance: &Option<Decimal>,
    deposits: &[Coin],
    pools: &[Coin],
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
        let deposits: Vec<Uint256> = deposits.iter().map(|coin| coin.amount.into()).collect();
        let pools: Vec<Uint256> = pools.iter().map(|coin| coin.amount.into()).collect();

        // Ensure each prices are not dropped as much as slippage tolerance rate
        match pool_type {
            PoolType::StableSwap { .. } => {
                // TODO: shouldn't be necessary to handle unwraps properly as they come from Uint128, but doublecheck!
                let pools_total: Uint256 = pools
                    .into_iter()
                    .fold(Uint256::zero(), |acc, x| acc.checked_add(x).unwrap());
                let deposits_total: Uint256 = deposits
                    .into_iter()
                    .fold(Uint256::zero(), |acc, x| acc.checked_add(x).unwrap());

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
                if deposits.len() != 2 || pools.len() != 2 {
                    return Err(ContractError::InvalidPoolAssetsLength {
                        expected: 2,
                        actual: deposits.len(),
                    });
                }
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

// TODO: handle unwraps properly
#[allow(clippy::unwrap_used)]
pub fn compute_d(amp_factor: &u64, deposits: &[Coin]) -> Option<Uint256> {
    let n_coins = Uint128::from(deposits.len() as u128);

    // sum(x_i), a.k.a S
    let sum_x = deposits
        .iter()
        .fold(Uint128::zero(), |acc, x| acc.checked_add(x.amount).unwrap());

    if sum_x == Uint128::zero() {
        Some(Uint256::zero())
    } else {
        // do as below but for a generic number of assets
        let amount_times_coins: Vec<Uint128> = deposits
            .iter()
            .map(|coin| coin.amount.checked_mul(n_coins).unwrap())
            .collect();

        // Newton's method to approximate D
        let mut d_prev: Uint256;
        let mut d: Uint256 = sum_x.into();
        for _ in 0..256 {
            let mut d_prod = d;
            for amount in amount_times_coins.clone().into_iter() {
                d_prod = d_prod
                    .checked_mul(d)
                    .unwrap()
                    .checked_div(amount.into())
                    .unwrap();
            }
            d_prev = d;
            d = compute_next_d(amp_factor, d, d_prod, sum_x, n_coins).unwrap();
            // Equality with the precision of 1
            if d > d_prev {
                if d.checked_sub(d_prev).unwrap() <= Uint256::one() {
                    break;
                }
            } else if d_prev.checked_sub(d).unwrap() <= Uint256::one() {
                break;
            }
        }

        Some(d)
    }
}

// TODO: handle unwraps properly
#[allow(clippy::unwrap_used)]
fn compute_next_d(
    amp_factor: &u64,
    d_init: Uint256,
    d_prod: Uint256,
    sum_x: Uint128,
    n_coins: Uint128,
) -> Option<Uint256> {
    let ann = amp_factor.checked_mul(n_coins.u128() as u64)?;
    let leverage = Uint256::from(sum_x).checked_mul(ann.into()).unwrap();
    // d = (ann * sum_x + d_prod * n_coins) * d / ((ann - 1) * d + (n_coins + 1) * d_prod)
    let numerator = d_init
        .checked_mul(
            d_prod
                .checked_mul(n_coins.into())
                .unwrap()
                .checked_add(leverage)
                .unwrap(),
        )
        .unwrap();
    let denominator = d_init
        .checked_mul(ann.checked_sub(1)?.into())
        .unwrap()
        .checked_add(
            d_prod
                .checked_mul((n_coins.checked_add(1u128.into()).unwrap()).into())
                .unwrap(),
        )
        .unwrap();
    Some(numerator.checked_div(denominator).unwrap())
}

/// Computes the amount of pool tokens to mint after a deposit.
#[allow(clippy::unwrap_used, clippy::too_many_arguments)]
pub fn compute_mint_amount_for_deposit(
    amp_factor: &u64,
    deposits: &[Coin],
    swaps: &[Coin],
    pool_token_supply: Uint128,
) -> Option<Uint128> {
    // Initial invariant
    let d_0 = compute_d(amp_factor, deposits)?;

    let new_balances: Vec<Coin> = swaps
        .iter()
        .enumerate()
        .map(|(i, pool_asset)| {
            let deposit_amount = deposits[i].amount;
            let new_amount = pool_asset.amount.checked_add(deposit_amount).unwrap();
            Coin {
                denom: pool_asset.denom.clone(),
                amount: new_amount,
            }
        })
        .collect();

    // Invariant after change
    let d_1 = compute_d(amp_factor, &new_balances)?;
    if d_1 <= d_0 {
        None
    } else {
        let amount = Uint256::from(pool_token_supply)
            .checked_mul(d_1.checked_sub(d_0).unwrap())
            .unwrap()
            .checked_div(d_0)
            .unwrap();
        Some(Uint128::try_from(amount).unwrap())
    }
}

/// Compute the swap amount `y` in proportion to `x`.
///
/// Solve for `y`:
///
/// ```text
/// y**2 + y * (sum' - (A*n**n - 1) * D / (A * n**n)) = D ** (n + 1) / (n ** (2 * n) * prod' * A)
/// y**2 + b*y = c
/// ```
#[allow(clippy::many_single_char_names, clippy::unwrap_used)]
pub fn compute_y_raw(
    n_coins: u8,
    amp_factor: &u64,
    swap_in: Uint128,
    //swap_out: Uint128,
    no_swap: Uint128,
    d: Uint256,
) -> Option<Uint256> {
    let ann = amp_factor.checked_mul(n_coins.into())?; // A * n ** n

    // sum' = prod' = x
    // c =  D ** (n + 1) / (n ** (2 * n) * prod' * A)
    let mut c = d;

    c = c
        .checked_mul(d)
        .unwrap()
        .checked_div(swap_in.checked_mul(n_coins.into()).unwrap().into())
        .unwrap();

    c = c
        .checked_mul(d)
        .unwrap()
        .checked_div(no_swap.checked_mul(n_coins.into()).unwrap().into())
        .unwrap();
    c = c
        .checked_mul(d)
        .unwrap()
        .checked_div(ann.checked_mul(n_coins.into()).unwrap().into())
        .unwrap();
    // b = sum(swap_in, no_swap) + D // Ann - D
    // not subtracting D here because that could result in a negative.
    let b = d
        .checked_div(ann.into())
        .unwrap()
        .checked_add(swap_in.into())
        .unwrap()
        .checked_add(no_swap.into())
        .unwrap();

    // Solve for y by approximating: y**2 + b*y = c
    let mut y_prev: Uint256;
    let mut y = d;
    for _ in 0..1000 {
        y_prev = y;
        // y = (y * y + c) / (2 * y + b - d);
        let y_numerator = y.checked_mul(y).unwrap().checked_add(c).unwrap();
        let y_denominator = y
            .checked_mul(Uint256::from(2u8))
            .unwrap()
            .checked_add(b)
            .unwrap()
            .checked_sub(d)
            .unwrap();
        y = y_numerator.checked_div(y_denominator).unwrap();
        if y > y_prev {
            if y.checked_sub(y_prev).unwrap() <= Uint256::one() {
                break;
            }
        } else if y_prev.checked_sub(y).unwrap() <= Uint256::one() {
            break;
        }
    }
    Some(y)
}

/// Computes the swap amount `y` in proportion to `x`.
#[allow(clippy::unwrap_used)]
pub fn compute_y(
    n_coins: u8,
    amp_factor: &u64,
    x: Uint128,
    no_swap: Uint128,
    d: Uint256,
) -> Option<Uint128> {
    let amount = compute_y_raw(n_coins, amp_factor, x, no_swap, d)?;
    Some(Uint128::try_from(amount).unwrap())
}

/// Compute SwapResult after an exchange
#[allow(clippy::unwrap_used)]
pub fn swap_to(
    n_coins: u8,
    amp_factor: &u64,
    source_amount: Uint128,
    swap_source_amount: Uint128,
    swap_destination_amount: Uint128,
    unswaped_amount: Uint128,
) -> Option<SwapResult> {
    let deposits = vec![
        coin(swap_source_amount.u128(), "denom1"),
        coin(swap_destination_amount.u128(), "denom2"),
        coin(unswaped_amount.u128(), "denom3"),
    ];
    let y = compute_y(
        n_coins,
        amp_factor,
        swap_source_amount.checked_add(source_amount).unwrap(),
        unswaped_amount,
        compute_d(amp_factor, &deposits).unwrap(),
    )?;
    // https://github.com/curvefi/curve-contract/blob/b0bbf77f8f93c9c5f4e415bce9cd71f0cdee960e/contracts/pool-templates/base/SwapTemplateBase.vy#L466
    let dy = swap_destination_amount
        .checked_sub(y)
        .unwrap()
        .checked_sub(Uint128::one())
        .unwrap();

    let amount_swapped = dy;
    let new_destination_amount = swap_destination_amount.checked_sub(amount_swapped).unwrap();
    let new_source_amount = swap_source_amount.checked_add(source_amount).unwrap();

    Some(SwapResult {
        new_source_amount,
        new_destination_amount,
        amount_swapped,
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::arithmetic_side_effects,
    clippy::too_many_arguments
)]
mod tests {
    use cosmwasm_std::coin;
    use proptest::prelude::*;
    use rand::Rng;
    use sim::Model;

    use super::*;

    /// Minimum amplification coefficient.
    pub const MIN_AMP: u64 = 1;

    /// Maximum amplification coefficient.
    pub const MAX_AMP: u64 = 1_000_000;

    /// Maximum number of tokens to swap at once.
    pub const MAX_TOKENS_IN: Uint128 = Uint128::new(2u128 << 110);

    /// Number of coins in a swap. Hardcoded to 3 to reuse previous tests
    pub const N_COINS: u8 = 3;

    fn check_d(model: &Model, amount_a: u128, amount_b: u128, amount_c: u128) -> Uint256 {
        let deposits = vec![
            coin(amount_a, "denom1"),
            coin(amount_b, "denom2"),
            coin(amount_c, "denom4"),
        ];

        let d = compute_d(&model.amp_factor, &deposits).unwrap();
        d
    }

    fn check_y(model: &Model, swap_in: u128, no_swap: u128, d: Uint256) {
        let y = compute_y_raw(
            N_COINS,
            &model.amp_factor,
            Uint128::new(swap_in),
            Uint128::new(no_swap),
            d,
        )
        .unwrap();
        assert_eq!(
            Uint128::try_from(y).unwrap().u128(),
            model.sim_y(0, 1, swap_in)
        )
    }

    #[test]
    fn test_curve_math_specific() {
        // Specific cases
        let model_no_balance = Model::new(1, vec![0, 0, 0], N_COINS);
        check_d(&model_no_balance, 0, 0, 0);

        let amount_a = 1046129065254161082u128;
        let amount_b = 1250710035549196829u128;
        let amount_c = 1111111111111111111u128;
        let model = Model::new(1188, vec![amount_a, amount_b, amount_c], N_COINS);
        let d = check_d(&model, amount_a, amount_b, amount_c);
        let amount_x = 2045250484898639148u128;
        check_y(&model, amount_x, amount_c, d);

        let amount_a = 862538457714585493u128;
        let amount_b = 492548187909826733u128;
        let amount_c = 777777777777777777u128;
        let model = Model::new(9, vec![amount_a, amount_b, amount_c], N_COINS);
        let d = check_d(&model, amount_a, amount_b, amount_c);
        let amount_x = 815577754938955939u128;

        check_y(&model, amount_x, amount_c, d);
    }

    #[test]
    fn test_compute_mint_amount_for_deposit() {
        let deposits = vec![
            coin(MAX_TOKENS_IN.u128(), "denom1"),
            coin(MAX_TOKENS_IN.u128(), "denom2"),
            coin(MAX_TOKENS_IN.u128(), "denom4"),
        ];

        let pool_assets = vec![
            coin(MAX_TOKENS_IN.u128(), "denom1"),
            coin(MAX_TOKENS_IN.u128(), "denom2"),
            coin(MAX_TOKENS_IN.u128(), "denom4"),
        ];

        let pool_token_supply = MAX_TOKENS_IN;

        let actual_mint_amount =
            compute_mint_amount_for_deposit(&MIN_AMP, &deposits, &pool_assets, pool_token_supply)
                .unwrap();
        let expected_mint_amount = MAX_TOKENS_IN;

        assert_eq!(actual_mint_amount, expected_mint_amount);
    }

    #[ignore]
    #[test]
    fn test_curve_math_with_random_inputs() {
        for _ in 0..100 {
            let mut rng = rand::thread_rng();

            let amp_factor: u64 = rng.gen_range(MIN_AMP..=MAX_AMP);
            let amount_a = rng.gen_range(1..=MAX_TOKENS_IN.u128());
            let amount_b = rng.gen_range(1..=MAX_TOKENS_IN.u128());
            let amount_c = rng.gen_range(1..=MAX_TOKENS_IN.u128());
            println!("testing curve_math_with_random_inputs:");
            println!(
                "amp_factor: {}, amount_a: {}, amount_b: {}, amount_c: {}",
                amp_factor, amount_a, amount_b, amount_c,
            );

            let model = Model::new(amp_factor, vec![amount_a, amount_b, amount_c], N_COINS);
            let d = check_d(&model, amount_a, amount_b, amount_c);
            let amount_x = rng.gen_range(0..=amount_a);

            println!("amount_x: {}", amount_x);
            check_y(&model, amount_x, amount_c, d);
        }
    }

    #[derive(Debug)]
    struct SwapTest {
        pub amp_factor: u64,
        pub swap_reserve_balance_a: Uint128,
        pub swap_reserve_balance_b: Uint128,
        pub swap_reserve_balance_c: Uint128,
        pub user_token_balance_a: Uint128,
        pub user_token_balance_b: Uint128,
    }

    impl SwapTest {
        pub fn swap_a_to_b(&mut self, swap_amount: Uint128) {
            self.do_swap(true, swap_amount)
        }

        pub fn swap_b_to_a(&mut self, swap_amount: Uint128) {
            self.do_swap(false, swap_amount)
        }

        fn do_swap(&mut self, swap_a_to_b: bool, source_amount: Uint128) {
            let (swap_source_amount, swap_dest_amount) = match swap_a_to_b {
                true => (self.swap_reserve_balance_a, self.swap_reserve_balance_b),
                false => (self.swap_reserve_balance_b, self.swap_reserve_balance_a),
            };

            let SwapResult {
                new_source_amount,
                new_destination_amount,
                amount_swapped,
                ..
            } = swap_to(
                N_COINS,
                &self.amp_factor,
                source_amount,
                swap_source_amount,
                swap_dest_amount,
                self.swap_reserve_balance_c,
            )
            .unwrap();

            match swap_a_to_b {
                true => {
                    self.swap_reserve_balance_a = new_source_amount;
                    self.swap_reserve_balance_b = new_destination_amount;
                    self.user_token_balance_a -= source_amount;
                    self.user_token_balance_b += amount_swapped;
                }
                false => {
                    self.swap_reserve_balance_a = new_destination_amount;
                    self.swap_reserve_balance_b = new_source_amount;
                    self.user_token_balance_a += amount_swapped;
                    self.user_token_balance_b -= source_amount;
                }
            }
        }
    }

    proptest! {
        #[test]
        fn test_swaps_does_not_result_in_more_tokens(
            amp_factor in MIN_AMP..=MAX_AMP,
            initial_user_token_a_amount in 10_000_000..MAX_TOKENS_IN.u128() >> 16,
            initial_user_token_b_amount in 10_000_000..MAX_TOKENS_IN.u128() >> 16,
        ) {

            let mut t = SwapTest { amp_factor, swap_reserve_balance_a: MAX_TOKENS_IN, swap_reserve_balance_b: MAX_TOKENS_IN,
                swap_reserve_balance_c: MAX_TOKENS_IN,
                user_token_balance_a: Uint128::new(initial_user_token_a_amount),
                user_token_balance_b:Uint128::new(initial_user_token_b_amount),
                };

            const ITERATIONS: u64 = 100;
            const SHRINK_MULTIPLIER: u64= 10;

            for i in 0..ITERATIONS {
                let before_balance_a = t.user_token_balance_a;
                let before_balance_b = t.user_token_balance_b;
                let swap_amount = before_balance_a / Uint128::from((i + 1) * SHRINK_MULTIPLIER);
                t.swap_a_to_b(swap_amount);
                let after_balance = t.user_token_balance_a + t.user_token_balance_b;

                assert!(before_balance_a + before_balance_b >= after_balance, "before_a: {}, before_b: {}, after_a: {}, after_b: {}, amp_factor: {:?}", before_balance_a, before_balance_b, t.user_token_balance_a, t.user_token_balance_b, amp_factor);
            }

            for i in 0..ITERATIONS {
                let before_balance_a = t.user_token_balance_a;
                let before_balance_b = t.user_token_balance_b;
                let swap_amount = before_balance_a / Uint128::from((i + 1) * SHRINK_MULTIPLIER);
                t.swap_a_to_b(swap_amount);
                let after_balance = t.user_token_balance_a + t.user_token_balance_b;

                assert!(before_balance_a + before_balance_b >= after_balance, "before_a: {}, before_b: {}, after_a: {}, after_b: {}, amp_factor: {:?}", before_balance_a, before_balance_b, t.user_token_balance_a, t.user_token_balance_b, amp_factor);
            }
        }
    }

    #[test]
    fn test_swaps_does_not_result_in_more_tokens_specific_one() {
        const AMP_FACTOR: u64 = 324449;
        const INITIAL_SWAP_RESERVE_AMOUNT: Uint128 = Uint128::new(100_000_000_000u128);
        const INITIAL_USER_TOKEN_AMOUNT: Uint128 = Uint128::new(10_000_000_000u128);

        let mut t = SwapTest {
            amp_factor: AMP_FACTOR,
            swap_reserve_balance_a: INITIAL_SWAP_RESERVE_AMOUNT,
            swap_reserve_balance_b: INITIAL_SWAP_RESERVE_AMOUNT,
            swap_reserve_balance_c: INITIAL_SWAP_RESERVE_AMOUNT,
            user_token_balance_a: INITIAL_USER_TOKEN_AMOUNT,
            user_token_balance_b: INITIAL_USER_TOKEN_AMOUNT,
        };

        t.swap_a_to_b(Uint128::new(2097152u128));
        t.swap_a_to_b(Uint128::new(8053063680u128));
        t.swap_a_to_b(Uint128::new(48u128));
        assert!(
            t.user_token_balance_a + t.user_token_balance_b
                <= INITIAL_USER_TOKEN_AMOUNT * Uint128::from(2u8)
        );
    }

    #[test]
    fn test_swaps_does_not_result_in_more_tokens_specific_two() {
        const AMP_FACTOR: u64 = 186512;
        const INITIAL_SWAP_RESERVE_AMOUNT: Uint128 = Uint128::new(100_000_000_000u128);
        const INITIAL_USER_TOKEN_AMOUNT: Uint128 = Uint128::new(1_000_000_000u128);

        let mut t = SwapTest {
            amp_factor: AMP_FACTOR,
            swap_reserve_balance_a: INITIAL_SWAP_RESERVE_AMOUNT,
            swap_reserve_balance_b: INITIAL_SWAP_RESERVE_AMOUNT,
            swap_reserve_balance_c: INITIAL_SWAP_RESERVE_AMOUNT,
            user_token_balance_a: INITIAL_USER_TOKEN_AMOUNT,
            user_token_balance_b: INITIAL_USER_TOKEN_AMOUNT,
        };

        t.swap_b_to_a(Uint128::new(33579101u128));
        t.swap_a_to_b(Uint128::new(2097152u128));
        assert!(
            t.user_token_balance_a + t.user_token_balance_b
                <= INITIAL_USER_TOKEN_AMOUNT * Uint128::from(2u8)
        );
    }

    #[test]
    fn test_swaps_does_not_result_in_more_tokens_specific_three() {
        const AMP_FACTOR: u64 = 1220;
        const INITIAL_SWAP_RESERVE_AMOUNT: Uint128 = Uint128::new(100_000_000_000u128);
        const INITIAL_USER_TOKEN_AMOUNT: Uint128 = Uint128::new(1_000_000_000u128);

        let mut t = SwapTest {
            amp_factor: AMP_FACTOR,
            swap_reserve_balance_a: INITIAL_SWAP_RESERVE_AMOUNT,
            swap_reserve_balance_b: INITIAL_SWAP_RESERVE_AMOUNT,
            swap_reserve_balance_c: INITIAL_SWAP_RESERVE_AMOUNT,
            user_token_balance_a: INITIAL_USER_TOKEN_AMOUNT,
            user_token_balance_b: INITIAL_USER_TOKEN_AMOUNT,
        };

        t.swap_b_to_a(Uint128::from(65535u128));
        t.swap_b_to_a(Uint128::from(6133503u128));
        t.swap_a_to_b(Uint128::from(65535u128));
        assert!(
            t.user_token_balance_a + t.user_token_balance_b
                <= INITIAL_USER_TOKEN_AMOUNT * Uint128::from(2u8)
        );
    }

    proptest! {
        #[test]
        fn test_virtual_price_does_not_decrease_from_deposit(
            amp_factor in MIN_AMP..=MAX_AMP,
            deposit_amount_a in 0..MAX_TOKENS_IN.u128() >> 2,
            deposit_amount_b in 0..MAX_TOKENS_IN.u128() >> 2,
            deposit_amount_c in 0..MAX_TOKENS_IN.u128() >> 2,
            swap_token_a_amount in 0..MAX_TOKENS_IN.u128(),
            swap_token_b_amount in 0..MAX_TOKENS_IN.u128(),
            swap_token_c_amount in 0..MAX_TOKENS_IN.u128(),
            pool_token_supply in 0..MAX_TOKENS_IN.u128(),
        ) {
            let swaps = vec![
                coin(swap_token_a_amount, "denom1"),
                coin(swap_token_b_amount, "denom2"),
                coin(swap_token_c_amount, "denom3"),
            ];

            let d0 = compute_d(&amp_factor, &swaps).unwrap();

            let deposits = vec![
                coin(deposit_amount_a, "denom1"),
                coin(deposit_amount_b, "denom2"),
                coin(deposit_amount_c, "denom3"),
            ];

            let mint_amount = compute_mint_amount_for_deposit(
                &amp_factor,
                &swaps,
                &deposits,
                Uint128::new(pool_token_supply),
                );
            prop_assume!(mint_amount.is_some());

            let new_swap_token_a_amount = swap_token_a_amount + deposit_amount_a;
            let new_swap_token_b_amount = swap_token_b_amount + deposit_amount_b;
            let new_swap_token_c_amount = swap_token_c_amount + deposit_amount_c;
            let new_pool_token_supply = pool_token_supply + mint_amount.unwrap().u128();

            let new_swaps = vec![
                coin(new_swap_token_a_amount, "denom1"),
                coin(new_swap_token_b_amount, "denom2"),
                coin(new_swap_token_c_amount, "denom3"),
            ];

            let d1 = compute_d(&amp_factor, &new_swaps).unwrap();

            assert!(d0 < d1);
            assert!(d0 / Uint256::from( pool_token_supply) <= d1 /  Uint256::from( new_pool_token_supply));
        }
    }

    proptest! {
        #[test]
        fn test_virtual_price_does_not_decrease_from_swap(
            amp_factor in MIN_AMP..=MAX_AMP,
            source_token_amount in 0..MAX_TOKENS_IN.u128(),
            swap_source_amount in 0..MAX_TOKENS_IN.u128(),
            swap_destination_amount in 0..MAX_TOKENS_IN.u128(),
            unswapped_amount in 0..MAX_TOKENS_IN.u128(),
        ) {
            let source_token_amount = source_token_amount;
            let swap_source_amount = swap_source_amount;
            let swap_destination_amount = swap_destination_amount;
            let unswapped_amount = unswapped_amount;

            let deposits = vec![
                coin(swap_source_amount, "denom1"),
                coin(swap_destination_amount, "denom2"),
                coin(unswapped_amount, "denom3"),
            ];

            let d0 = compute_d(&amp_factor, &deposits).unwrap();

            let swap_result = swap_to(N_COINS, &amp_factor, source_token_amount.into(), swap_source_amount.into(), swap_destination_amount.into(), unswapped_amount.into());
            prop_assume!(swap_result.is_some());

            let swap_result = swap_result.unwrap();

            let swaps = vec![
                coin(swap_result.new_source_amount.u128(), "denom1"),
                coin(swap_result.new_destination_amount.u128(), "denom2"),
                coin(unswapped_amount, "denom3"),
            ];

            let d1 = compute_d(&amp_factor, &swaps).unwrap();

            assert!(d0 <= d1);  // Pool token supply not changed on swaps
        }
    }
}
