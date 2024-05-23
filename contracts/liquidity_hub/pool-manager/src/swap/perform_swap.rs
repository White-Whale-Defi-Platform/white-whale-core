use cosmwasm_std::{Coin, Decimal, DepsMut, Uint128, Uint256};

use white_whale_std::pool_manager::PoolInfo;
use white_whale_std::pool_network::swap::assert_max_spread;

use crate::helpers::{aggregate_outgoing_fees, get_asset_indexes_in_pool};
use crate::{
    helpers,
    state::{get_pool_by_identifier, POOLS},
    ContractError,
};

#[derive(Debug)]
pub struct SwapResult {
    /// The asset that should be returned to the user from the swap.
    pub return_asset: Coin,
    /// The burn fee of `return_asset` associated with this swap transaction.
    pub burn_fee_asset: Coin,
    /// The protocol fee of `return_asset` associated with this swap transaction.
    pub protocol_fee_asset: Coin,
    /// The swap fee of `return_asset` associated with this swap transaction.
    pub swap_fee_asset: Coin,
    /// The osmosis fee of `return_asset` associated with this swap transaction.
    #[cfg(feature = "osmosis")]
    pub osmosis_fee_asset: Coin,
    /// The pool that was traded.
    pub pool_info: PoolInfo,
    /// The amount of spread that occurred during the swap from the original exchange rate.
    pub spread_amount: Uint128,
}

/// Attempts to perform a swap from `offer_asset` to the relevant opposing
/// asset in the pool identified by `pool_identifier`.
///
/// Assumes that `offer_asset` is a **native token**.
///
/// The resulting [`SwapResult`] has actions that should be taken, as the swap has been performed.
/// In other words, the caller of the `perform_swap` function _should_ make use
/// of each field in [`SwapResult`] (besides fields like `spread_amount`).
pub fn perform_swap(
    deps: DepsMut,
    offer_asset: Coin,
    ask_asset_denom: String,
    pool_identifier: String,
    belief_price: Option<Decimal>,
    max_spread: Option<Decimal>,
) -> Result<SwapResult, ContractError> {
    let mut pool_info = get_pool_by_identifier(&deps.as_ref(), &pool_identifier)?;

    let (
        offer_asset_in_pool,
        ask_asset_in_pool,
        offer_index,
        ask_index,
        offer_decimal,
        ask_decimal,
    ) = get_asset_indexes_in_pool(&pool_info, offer_asset.denom, ask_asset_denom)?;

    // compute the swap
    let swap_computation = helpers::compute_swap(
        Uint256::from(pool_info.assets.len() as u128),
        offer_asset_in_pool.amount,
        ask_asset_in_pool.amount,
        offer_asset.amount,
        pool_info.pool_fees.clone(),
        &pool_info.pool_type,
        offer_decimal,
        ask_decimal,
    )?;

    let return_asset = Coin {
        denom: ask_asset_in_pool.denom.clone(),
        amount: swap_computation.return_amount,
    };

    // Assert spread and other operations
    // check max spread limit if exist
    assert_max_spread(
        belief_price,
        max_spread,
        offer_asset.amount,
        return_asset.amount,
        swap_computation.spread_amount,
    )?;

    // State changes to the pools balances
    {
        // add the offer amount to the pool
        pool_info.assets[offer_index].amount = pool_info.assets[offer_index]
            .amount
            .checked_add(offer_asset.amount)?;

        // Deduct the return amount and fees from the pool
        let outgoing_fees = aggregate_outgoing_fees(&swap_computation.to_simulation_response())?;

        pool_info.assets[ask_index].amount = pool_info.assets[ask_index]
            .amount
            .checked_sub(return_asset.amount)?
            .checked_sub(outgoing_fees)?;

        POOLS.save(deps.storage, &pool_identifier, &pool_info)?;
    }

    let burn_fee_asset = Coin {
        denom: ask_asset_in_pool.denom.clone(),
        amount: swap_computation.burn_fee_amount,
    };
    let protocol_fee_asset = Coin {
        denom: ask_asset_in_pool.denom.clone(),
        amount: swap_computation.protocol_fee_amount,
    };

    #[allow(clippy::redundant_clone)]
    let swap_fee_asset = Coin {
        denom: ask_asset_in_pool.denom.clone(),
        amount: swap_computation.swap_fee_amount,
    };

    #[cfg(not(feature = "osmosis"))]
    {
        Ok(SwapResult {
            return_asset,
            swap_fee_asset,
            burn_fee_asset,
            protocol_fee_asset,
            pool_info,
            spread_amount: swap_computation.spread_amount,
        })
    }

    #[cfg(feature = "osmosis")]
    {
        let osmosis_fee_asset = Coin {
            denom: ask_asset_in_pool.denom,
            amount: swap_computation.swap_fee_amount,
        };

        Ok(SwapResult {
            return_asset,
            swap_fee_asset,
            burn_fee_asset,
            protocol_fee_asset,
            osmosis_fee_asset,
            pool_info,
            spread_amount: swap_computation.spread_amount,
        })
    }
}
