use cosmwasm_std::{ensure, Coin, Decimal, DepsMut, Uint128};

use white_whale_std::pool_manager::PoolInfo;
use white_whale_std::pool_network::swap::assert_max_spread;

use crate::helpers::aggregate_outgoing_fees;
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
    let pools = &pool_info.assets;

    // Find the index of the offer and ask asset in the pools
    let offer_index = pools
        .iter()
        .position(|pool| offer_asset.denom == pool.denom)
        .ok_or(ContractError::AssetMismatch)?;
    let ask_index = pools
        .iter()
        .position(|pool| ask_asset_denom == pool.denom)
        .ok_or(ContractError::AssetMismatch)?;

    // make sure it's not the same asset
    ensure!(offer_index != ask_index, ContractError::AssetMismatch);

    let decimals = &pool_info.asset_decimals;

    let offer_asset_in_pool = pools[offer_index].clone();
    let ask_asset_in_pool = pools[ask_index].clone();
    let offer_decimal = decimals[offer_index];
    let ask_decimal = decimals[ask_index];

    let offer_amount = offer_asset.amount;
    let pool_fees = pool_info.pool_fees.clone();

    // compute the swap
    let swap_computation = helpers::compute_swap(
        offer_asset_in_pool.amount,
        ask_asset_in_pool.amount,
        offer_amount,
        pool_fees,
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
            .checked_add(offer_amount)?;

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
