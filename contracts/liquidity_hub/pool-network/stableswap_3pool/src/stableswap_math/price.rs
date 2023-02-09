//! Utilities for getting the virtual price of a pool.

use crate::stableswap_math::curve::StableSwap;
use cosmwasm_std::{Uint128, Uint256};

/// Utilities for calculating the virtual price of a WhiteWhale LP token.
///
/// This is especially useful for if you want to use a WhiteWhale LP token as collateral.
///
/// # Calculating liquidation value
///
/// To use a WhiteWhale LP token as collateral, you will need to fetch the prices
/// of both of the tokens in the pool and get the min of the two. Then,
/// use the [WhiteWhaleStableSwap::calculate_virtual_price_of_pool_tokens] function to
/// get the virtual price.
///
/// This virtual price is resilient to manipulations of the LP token price.
///
/// Hence, `min_lp_price = min_value * virtual_price`.
///
/// # Additional Reading
/// - [Chainlink: Using Chainlink Oracles to Securely Utilize Curve LP Pools](https://blog.chain.link/using-chainlink-oracles-to-securely-utilize-curve-lp-pools/)
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct WhiteWhaleStableSwap {
    /// Initial amp factor, or `A`.
    ///
    /// See [`StableSwap::compute_amp_factor`].
    pub initial_amp_factor: u64,
    /// Target amp factor, or `A`.
    ///
    /// See [`StableSwap::compute_amp_factor`].
    pub target_amp_factor: u64,
    /// Current timestmap.
    pub current_ts: i64,
    /// Start ramp timestamp for calculating the amp factor, or `A`.
    ///
    /// See [`StableSwap::compute_amp_factor`].
    pub start_ramp_ts: i64,
    /// Stop ramp timestamp for calculating the amp factor, or `A`.
    ///
    /// See [`StableSwap::compute_amp_factor`].
    pub stop_ramp_ts: i64,

    /// Total supply of LP tokens.
    ///
    /// This is `pool_mint.supply`, where `pool_mint` is an SPL Token Mint.
    pub lp_mint_supply: u64,
    /// Amount of token A.
    ///
    /// This is `token_a.reserve.amount`, where `token_a.reserve` is an SPL Token Token Account.
    pub token_a_reserve: Uint128,
    /// Amount of token B.
    ///
    /// This is `token_b.reserve.amount`, where `token_b.reserve` is an SPL Token Token Account.
    pub token_b_reserve: Uint128,
    /// Amount of token C.
    ///
    /// This is `token_c.reserve.amount`, where `token_c.reserve` is an SPL Token Token Account.
    pub token_c_reserve: Uint128,
}

impl From<&WhiteWhaleStableSwap> for StableSwap {
    fn from(swap: &WhiteWhaleStableSwap) -> Self {
        crate::stableswap_math::curve::StableSwap::new(
            swap.initial_amp_factor,
            swap.target_amp_factor,
            swap.current_ts,
            swap.start_ramp_ts,
            swap.stop_ramp_ts,
        )
    }
}

impl WhiteWhaleStableSwap {
    /// Calculates the amount of pool tokens represented by the given amount of virtual tokens.
    ///
    /// A virtual token is the denomination of virtual price. For example, if there is a virtual price of 1.04
    /// on USDC-USDT LP, then 1 virtual token maps to 1/1.04 USDC-USDT LP tokens.
    ///
    /// This is useful for building assets that are backed by LP tokens.
    /// An example of this is [Cashio](https://github.com/CashioApp/cashio), which
    /// allows users to mint $CASH tokens based on the virtual price of underlying LP tokens.
    ///
    /// # Arguments
    ///
    /// - `virtual_amount` - The number of "virtual" underlying tokens.
    #[allow(clippy::unwrap_used)]
    pub fn calculate_pool_tokens_from_virtual_amount(
        &self,
        virtual_amount: Uint128,
    ) -> Option<Uint128> {
        let amount = Uint256::from(virtual_amount)
            .checked_mul(self.lp_mint_supply.into())
            .unwrap()
            .checked_div(self.compute_d().unwrap())
            .unwrap();
        Some(Uint128::try_from(amount).unwrap())
    }

    /// Calculates the virtual price of the given amount of pool tokens.
    ///
    /// The virtual price is defined as the current price of the pool LP token
    /// relative to the underlying pool assets.
    ///
    /// The virtual price in the StableSwap algorithm is obtained through taking the invariance
    /// of the pool, which by default takes every token as valued at 1.00 of the underlying.
    /// You can get the virtual price of each pool by calling this function
    /// for it.[^chainlink]
    ///
    /// [^chainlink]: Source: <https://blog.chain.link/using-chainlink-oracles-to-securely-utilize-curve-lp-pools/>
    #[allow(clippy::unwrap_used)]
    pub fn calculate_virtual_price_of_pool_tokens(
        &self,
        pool_token_amount: Uint128,
    ) -> Option<Uint128> {
        let amount = self
            .compute_d()?
            .checked_mul(pool_token_amount.into())
            .unwrap()
            .checked_div(self.lp_mint_supply.into())
            .unwrap();
        Some(Uint128::try_from(amount).unwrap())
    }

    /// Computes D, which is the virtual price times the total supply of the pool.
    #[allow(clippy::unwrap_used)]
    pub fn compute_d(&self) -> Option<Uint256> {
        let calculator = StableSwap::from(self);
        calculator.compute_d(
            self.token_a_reserve,
            self.token_b_reserve,
            self.token_c_reserve,
        )
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use proptest::prelude::*;

    use super::WhiteWhaleStableSwap;
    /*  TODO commented out proptests
       prop_compose! {
           fn arb_swap_unsafe()(
               token_a_reserve in 1_u64..=u64::MAX,
               token_b_reserve in 1_u64..=u64::MAX,
               token_c_reserve in 1_u64..=u64::MAX,
               lp_mint_supply in 1_u64..=u64::MAX
           ) -> WhiteWhaleStableSwap {
               WhiteWhaleStableSwap {
                   initial_amp_factor: 1,
                   target_amp_factor: 1,
                   current_ts: 1,
                   start_ramp_ts: 1,
                   stop_ramp_ts: 1,

                   lp_mint_supply,
                   token_a_reserve,
                   token_b_reserve,
                   token_c_reserve
               }
           }
       }

       prop_compose! {
           #[allow(clippy::integer_arithmetic)]
           fn arb_token_amount(decimals: u8)(
               amount in 1_u64..=(u64::MAX / 10u64.pow(decimals.into())),
           ) -> u64 {
               amount
           }
       }

       prop_compose! {
           fn arb_swap_reserves()(
               decimals in 0_u8..=19_u8,
               swap in arb_swap_unsafe()
           ) (
               token_a_reserve in arb_token_amount(decimals),
               token_b_reserve in arb_token_amount(decimals),
               swap in Just(swap)
           ) -> WhiteWhaleStableSwap {
               WhiteWhaleStableSwap {
                   token_a_reserve,
                   token_b_reserve,
                   ..swap
               }
           }
       }

       prop_compose! {
           fn arb_swap()(
               swap in arb_swap_reserves()
           ) (
               // targeting a maximum virtual price of 4
               // anything higher than this is a bit ridiculous
               lp_mint_supply in 1_u64.max((swap.token_a_reserve.min(swap.token_b_reserve)) / 4)..=(swap.token_a_reserve.checked_add(swap.token_b_reserve).unwrap_or(u64::MAX)),
               swap in Just(swap)
           ) -> WhiteWhaleStableSwap {
               WhiteWhaleStableSwap {
                   lp_mint_supply,
                   ..swap
               }
           }
       }

       proptest! {
         #[test]
         fn test_invertible(
             swap in arb_swap(),
             amount in 0_u64..=u64::MAX
         ) {
           let maybe_virt = swap.calculate_virtual_price_of_pool_tokens(amount);
           if maybe_virt.is_none() {
               // ignore virt calculation failures, since they won't be used in production
               return Ok(());
           }
           let virt = maybe_virt.unwrap();
           if virt == 0 {
               // this case doesn't matter because it's a noop.
               return Ok(());
           }

           let result_lp = swap.calculate_pool_tokens_from_virtual_amount(virt).unwrap();

           // tokens should never be created.
           prop_assert!(result_lp <= amount);

           // these numbers should be very close to each other.
           prop_assert!(1.0_f64 - (result_lp as f64) / (amount as f64) < 0.001_f64);
         }
       }

    */
}
