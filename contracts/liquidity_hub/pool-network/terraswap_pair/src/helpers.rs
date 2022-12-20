use std::cmp::Ordering;
use std::ops::Mul;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Decimal256, StdError, StdResult, Storage, Uint128, Uint256};
use cw_storage_plus::Item;

use terraswap::asset::{Asset, AssetInfo};
use terraswap::pair::PoolFee;

use crate::error::ContractError;

pub fn compute_swap(
    offer_pool_asset: Asset,
    ask_pool_asset: Asset,
    offer_amount: Uint128,
    pool_fees: PoolFee,
    collect_protocol_fees_in: Option<AssetInfo>,
) -> StdResult<SwapComputation> {
    let offer_pool: Uint256 = offer_pool_asset.amount.into();
    let ask_pool: Uint256 = ask_pool_asset.amount.into();
    let mut offer_amount: Uint256 = offer_amount.into();

    let mut protocol_fee_amount: Uint256 = Uint256::zero();

    // check if protocol fees are collected in offer_asset
    if is_protocol_fee_collected_in_asset(&collect_protocol_fees_in, &offer_pool_asset.info) {
        protocol_fee_amount = pool_fees.protocol_fee.compute(offer_amount);
        offer_amount -= protocol_fee_amount;
    }

    // offer => ask
    // ask_amount = (ask_pool * offer_amount / (offer_pool + offer_amount)) - swap_fee - protocol_fee
    let mut return_amount: Uint256 = Uint256::one()
        * Decimal256::from_ratio(ask_pool.mul(offer_amount), offer_pool + offer_amount);

    // calculate spread, swap and protocol fees
    let exchange_rate = Decimal256::from_ratio(ask_pool, offer_pool);
    let spread_amount: Uint256 = (offer_amount * exchange_rate) - return_amount;

    let swap_fee_amount: Uint256 = pool_fees.swap_fee.compute(return_amount);
    let burn_fee_amount: Uint256 = pool_fees.burn_fee.compute(return_amount);

    // protocol fee is subtracted from the return_amount if the fee is collected in the ask asset
    if !is_protocol_fee_collected_in_asset(&collect_protocol_fees_in, &offer_pool_asset.info) {
        protocol_fee_amount = pool_fees.protocol_fee.compute(return_amount);
        return_amount -= protocol_fee_amount;
    }

    // swap fee will be absorbed by the pool. Burn fee amount will be burned on a subsequent msg. The
    return_amount = return_amount - swap_fee_amount - burn_fee_amount;

    Ok(SwapComputation {
        return_asset: Asset {
            info: ask_pool_asset.info.clone(),
            amount: return_amount.try_into()?,
        },
        spread_amount: spread_amount.try_into()?,
        swap_fee_asset: Asset {
            info: ask_pool_asset.info.clone(),
            amount: swap_fee_amount.try_into()?,
        },
        protocol_fee_asset: Asset {
            info: collect_protocol_fees_in.unwrap_or_else(|| ask_pool_asset.info.clone()),
            amount: protocol_fee_amount.try_into()?,
        },
        burn_fee_asset: Asset {
            info: ask_pool_asset.info,
            amount: burn_fee_amount.try_into()?,
        },
    })
}

fn is_protocol_fee_collected_in_asset(
    collect_protocol_fees_in: &Option<AssetInfo>,
    asset_info: &AssetInfo,
) -> bool {
    if let Some(collect_protocol_fees_in) = collect_protocol_fees_in {
        return collect_protocol_fees_in == asset_info;
    }

    false
}

/// Represents the swap computation values
#[cw_serde]
pub struct SwapComputation {
    pub return_asset: Asset,
    pub spread_amount: Uint128,
    pub swap_fee_asset: Asset,
    pub protocol_fee_asset: Asset,
    pub burn_fee_asset: Asset,
}

pub fn compute_offer_amount(
    offer_pool_asset: Asset,
    ask_pool_asset: Asset,
    ask_amount: Uint128,
    pool_fees: PoolFee,
    collect_protocol_fees_in: Option<AssetInfo>,
) -> StdResult<OfferAmountComputation> {
    let offer_pool: Uint256 = offer_pool_asset.amount.into();
    let ask_pool: Uint256 = ask_pool_asset.amount.into();
    let ask_amount: Uint256 = ask_amount.into();

    // ask => offer

    // if fees are collected in the ask asset:
    // fees = protocol_fee + swap_fee + burn_fee
    // offer_amount = cp / (ask_pool - ask_amount / (1 - fees)) - offer_pool
    // if fees are collected in the offer asset:
    // fees = swap_fee + burn_fee
    // offer_amount = cp / (ask_pool - ask_amount / (1 - fees)) - offer_pool + protocol_fee
    let mut fees = pool_fees.swap_fee.to_decimal_256() + pool_fees.burn_fee.to_decimal_256();
    let mut protocol_fee_amount: Uint256 = Uint256::zero();

    // if fees are collected in the ask asset, add protocol fees to the fees amount
    if !is_protocol_fee_collected_in_asset(&collect_protocol_fees_in, &offer_pool_asset.info) {
        fees += pool_fees.protocol_fee.to_decimal_256();
    }

    let one_minus_commission = Decimal256::one() - fees;
    let inv_one_minus_commission = Decimal256::one() / one_minus_commission;

    let cp: Uint256 = offer_pool * ask_pool;
    let mut offer_amount: Uint256 = Uint256::one()
        .multiply_ratio(cp, ask_pool - ask_amount * inv_one_minus_commission)
        - offer_pool;

    let before_commission_deduction: Uint256 = ask_amount * inv_one_minus_commission;
    let spread_amount = (offer_amount * Decimal256::from_ratio(ask_pool, offer_pool))
        .saturating_sub(before_commission_deduction);

    // if fees are collected in the offer asset, recalculate the offer amount adding the protocol fee to it
    // offer_amount = cp / (ask_pool - ask_amount / (1 - fees)) - offer_pool + protocol_fee
    // offer_amount = x + (offer_amount * (1 - protocol_fee))
    // offer_amount = x / (1 - protocol_fee)
    if is_protocol_fee_collected_in_asset(&collect_protocol_fees_in, &offer_pool_asset.info) {
        let one_minus_protocol_fees = Decimal256::one() - pool_fees.protocol_fee.to_decimal_256();
        let inv_one_minus_protocol_fees = Decimal256::one() / one_minus_protocol_fees;
        offer_amount = offer_amount * inv_one_minus_protocol_fees;
        protocol_fee_amount = pool_fees.protocol_fee.compute(offer_amount);
    }

    let swap_fee_amount: Uint256 = pool_fees.swap_fee.compute(before_commission_deduction);
    let burn_fee_amount: Uint256 = pool_fees.burn_fee.compute(before_commission_deduction);

    if !is_protocol_fee_collected_in_asset(&collect_protocol_fees_in, &offer_pool_asset.info) {
        // if collecting protocol fees on ask asset
        protocol_fee_amount = pool_fees.protocol_fee.compute(before_commission_deduction);
    }

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
/// we compute new spread else we just use terraswap
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
        let spread_amount = if expected_return > return_amount {
            expected_return - return_amount
        } else {
            Uint256::zero()
        };

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
