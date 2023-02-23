use std::cmp::Ordering;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Decimal256, Deps, StdError, StdResult, Storage, Uint128, Uint256};
use cw_storage_plus::Item;

use terraswap::asset::{is_factory_token, Asset, AssetInfo};
use terraswap::querier::query_token_info;
use terraswap::trio::PoolFee;

use crate::error::ContractError;
use crate::stableswap_math::curve::StableSwap;

pub fn compute_swap(
    offer_pool: Uint128,
    ask_pool: Uint128,
    unswapped_pool: Uint128,
    offer_amount: Uint128,
    pool_fees: PoolFee,
    amp_factor: u64,
) -> StdResult<SwapComputation> {
    let invariant = StableSwap::new(amp_factor, amp_factor, 0, 0, 0);

    let result = invariant
        .swap_to(offer_amount, offer_pool, ask_pool, unswapped_pool)
        .unwrap();

    let return_amount: Uint256 = result.amount_swapped.into();
    let spread_amount = if Uint256::from(offer_amount) > return_amount {
        Uint256::from(offer_amount) - return_amount
    } else {
        return_amount - Uint256::from(offer_amount)
    };
    let swap_fee_amount: Uint256 = pool_fees.swap_fee.compute(return_amount);
    let protocol_fee_amount: Uint256 = pool_fees.protocol_fee.compute(return_amount);
    let burn_fee_amount: Uint256 = pool_fees.burn_fee.compute(return_amount);

    // swap and protocol fee will be absorbed by the pool. Burn fee amount will be burned on a subsequent msg.
    let return_amount: Uint256 =
        return_amount - swap_fee_amount - protocol_fee_amount - burn_fee_amount;

    Ok(SwapComputation {
        return_amount: return_amount.try_into()?,
        spread_amount: spread_amount.try_into()?,
        swap_fee_amount: swap_fee_amount.try_into()?,
        protocol_fee_amount: protocol_fee_amount.try_into()?,
        burn_fee_amount: burn_fee_amount.try_into()?,
    })
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
    unswapped_pool: Uint128,
    ask_amount: Uint128,
    pool_fees: PoolFee,
    amp_factor: u64,
) -> StdResult<OfferAmountComputation> {
    let fees = pool_fees.swap_fee.share + pool_fees.protocol_fee.share + pool_fees.burn_fee.share;
    let one_minus_commission = Decimal::one() - fees;
    let inv_one_minus_commission = Decimal::one() / one_minus_commission;

    let before_commission_deduction: Uint128 = ask_amount * inv_one_minus_commission;

    let invariant = StableSwap::new(amp_factor, amp_factor, 0, 0, 0);

    let offer_amount = invariant
        .reverse_sim(
            before_commission_deduction,
            offer_pool,
            ask_pool,
            unswapped_pool,
        )
        .unwrap();

    let spread_amount = if before_commission_deduction > offer_amount {
        before_commission_deduction - offer_amount
    } else {
        offer_amount - before_commission_deduction
    };

    let swap_fee_amount = pool_fees
        .swap_fee
        .compute(before_commission_deduction.into());
    let protocol_fee_amount = pool_fees
        .protocol_fee
        .compute(before_commission_deduction.into());
    let burn_fee_amount = pool_fees
        .burn_fee
        .compute(before_commission_deduction.into());

    Ok(OfferAmountComputation {
        offer_amount,
        spread_amount,
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
    deposits: &[Uint128; 3],
    pools: &[Asset; 3],
) -> Result<(), ContractError> {
    if let Some(slippage_tolerance) = *slippage_tolerance {
        let slippage_tolerance: Decimal256 = slippage_tolerance.into();
        if slippage_tolerance > Decimal256::one() {
            return Err(StdError::generic_err("slippage_tolerance cannot bigger than 1").into());
        }

        let one_minus_slippage_tolerance = Decimal256::one() - slippage_tolerance;
        let deposits: [Uint256; 3] = [deposits[0].into(), deposits[1].into(), deposits[2].into()];
        let pools: [Uint256; 3] = [
            pools[0].amount.into(),
            pools[1].amount.into(),
            pools[2].amount.into(),
        ];

        // Ensure each prices are not dropped as much as slippage tolerance rate
        //TODO three way slippage tolerance?
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
    asset_info_2: AssetInfo,
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
            Asset {
                info: asset_info_2,
                amount: Uint128::zero(),
            },
        ],
    )
}

/// Gets the total supply of the given liquidity token
pub fn get_total_share(deps: &Deps, liquidity_token: String) -> StdResult<Uint128> {
    let total_share = if is_factory_token(liquidity_token.as_str()) {
        //bank query total
        deps.querier.query_supply(&liquidity_token)?.amount
    } else {
        query_token_info(
            &deps.querier,
            deps.api.addr_validate(liquidity_token.as_str())?,
        )?
        .total_supply
    };
    Ok(total_share)
}
