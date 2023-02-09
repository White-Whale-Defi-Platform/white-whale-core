//! Swap calculations and curve invariant implementation
use cosmwasm_std::{Uint128, Uint256};

use num_traits::ToPrimitive;

/// Number of coins in a swap.
pub const N_COINS: u8 = 3;

/// Timestamp at 0
pub const ZERO_TS: i64 = 0;

/// Minimum ramp duration, in seconds.
pub const MIN_RAMP_DURATION: i64 = 86_400;

/// Minimum amplification coefficient.
pub const MIN_AMP: u64 = 1;

/// Maximum amplification coefficient.
pub const MAX_AMP: u64 = 1_000_000;

/// Maximum number of tokens to swap at once.
pub const MAX_TOKENS_IN: Uint128 = Uint128::new((2u128 << 60) - 1);

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

/// The [StableSwap] invariant calculator.
///
/// This is primarily used to calculate two quantities:
/// - `D`, the swap invariant, and
/// - `Y`, the amount of tokens swapped in an instruction.
///
/// This calculator also contains several helper utilities for computing
/// swap, withdraw, and deposit amounts.
///
/// # Resources:
///
/// - [Curve StableSwap paper](https://curve.fi/files/stableswap-paper.pdf)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StableSwap {
    /// Initial amplification coefficient (A)
    initial_amp_factor: u64,
    /// Target amplification coefficient (A)
    target_amp_factor: u64,
    /// Current unix timestamp
    current_ts: i64,
    /// Ramp A start timestamp
    start_ramp_ts: i64,
    /// Ramp A stop timestamp
    stop_ramp_ts: i64,
}

impl StableSwap {
    /// Constructs a new [StableSwap] invariant calculator.
    pub fn new(
        initial_amp_factor: u64,
        target_amp_factor: u64,
        current_ts: i64,
        start_ramp_ts: i64,
        stop_ramp_ts: i64,
    ) -> Self {
        Self {
            initial_amp_factor,
            target_amp_factor,
            current_ts,
            start_ramp_ts,
            stop_ramp_ts,
        }
    }

    #[allow(clippy::unwrap_used)]
    fn compute_next_d(
        &self,
        amp_factor: u64,
        d_init: Uint256,
        d_prod: Uint256,
        sum_x: Uint128,
    ) -> Option<Uint256> {
        let ann = amp_factor.checked_mul(N_COINS.into())?;
        let leverage = Uint256::from(sum_x).checked_mul(ann.into()).unwrap();
        // d = (ann * sum_x + d_prod * n_coins) * d / ((ann - 1) * d + (n_coins + 1) * d_prod)
        let numerator = d_init
            .checked_mul(
                d_prod
                    .checked_mul(N_COINS.into())
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
                    .checked_mul((N_COINS.checked_add(1)?).into())
                    .unwrap(),
            )
            .unwrap();
        Some(numerator.checked_div(denominator).unwrap())
    }

    /// Compute the amplification coefficient (A).
    ///
    /// The amplification coefficient is used to determine the slippage incurred when
    /// performing swaps. The lower it is, the closer the invariant is to the constant product[^stableswap].
    ///
    /// The amplication coefficient linearly increases with respect to time,
    /// based on the [`SwapInfo::start_ramp_ts`] and [`SwapInfo::stop_ramp_ts`] parameters.
    ///
    /// [^stableswap]: [Egorov, "StableSwap," 2019.](https://curve.fi/files/stableswap-paper.pdf)
    pub fn compute_amp_factor(&self) -> Option<u64> {
        if self.current_ts < self.stop_ramp_ts {
            let time_range = self.stop_ramp_ts.checked_sub(self.start_ramp_ts)?;
            let time_delta = self.current_ts.checked_sub(self.start_ramp_ts)?;

            // Compute amp factor based on ramp time
            if self.target_amp_factor >= self.initial_amp_factor {
                // Ramp up
                let amp_range = self
                    .target_amp_factor
                    .checked_sub(self.initial_amp_factor)?;
                let amp_delta = (amp_range as u128)
                    .checked_mul(time_delta.to_u128()?)?
                    .checked_div(time_range.to_u128()?)?
                    .to_u64()?;
                self.initial_amp_factor.checked_add(amp_delta)
            } else {
                // Ramp down
                let amp_range = self
                    .initial_amp_factor
                    .checked_sub(self.target_amp_factor)?;
                let amp_delta = (amp_range as u128)
                    .checked_mul(time_delta.to_u128()?)?
                    .checked_div(time_range.to_u128()?)?
                    .to_u64()?;
                self.initial_amp_factor.checked_sub(amp_delta)
            }
        } else {
            // when stop_ramp_ts == 0 or current_ts >= stop_ramp_ts
            Some(self.target_amp_factor)
        }
    }

    /// Computes the Stable Swap invariant (D).
    ///
    /// The invariant is defined as follows:
    ///
    /// ```text
    /// A * sum(x_i) * n**n + D = A * D * n**n + D**(n+1) / (n**n * prod(x_i))
    /// ```
    ///
    /// # Arguments
    ///
    /// - `amount_a` - The amount of token A owned by the LP pool. (i.e. token A reserves)
    /// - `amount_b` - The amount of token B owned by the LP pool. (i.e. token B reserves)
    /// - `amount_c` - The amount of token B owned by the LP pool. (i.e. token C reserves)
    ///
    #[allow(clippy::unwrap_used)]
    pub fn compute_d(
        &self,
        amount_a: Uint128,
        amount_b: Uint128,
        amount_c: Uint128,
    ) -> Option<Uint256> {
        let sum_x = amount_a
            .checked_add(amount_b.checked_add(amount_c).unwrap())
            .unwrap(); // sum(x_i), a.k.a S
        if sum_x == Uint128::zero() {
            Some(Uint256::zero())
        } else {
            let amp_factor = self.compute_amp_factor()?;
            let amount_a_times_coins = amount_a.checked_mul(N_COINS.into()).unwrap();
            let amount_b_times_coins = amount_b.checked_mul(N_COINS.into()).unwrap();
            let amount_c_times_coins = amount_c.checked_mul(N_COINS.into()).unwrap();

            // Newton's method to approximate D
            let mut d_prev: Uint256;
            let mut d: Uint256 = sum_x.into();
            for _ in 0..256 {
                let mut d_prod = d;
                d_prod = d_prod
                    .checked_mul(d)
                    .unwrap()
                    .checked_div(amount_a_times_coins.into())
                    .unwrap();
                d_prod = d_prod
                    .checked_mul(d)
                    .unwrap()
                    .checked_div(amount_b_times_coins.into())
                    .unwrap();
                d_prod = d_prod
                    .checked_mul(d)
                    .unwrap()
                    .checked_div(amount_c_times_coins.into())
                    .unwrap();
                d_prev = d;
                d = self.compute_next_d(amp_factor, d, d_prod, sum_x).unwrap();
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

    /// Computes the amount of pool tokens to mint after a deposit.
    #[allow(clippy::unwrap_used, clippy::too_many_arguments)]
    pub fn compute_mint_amount_for_deposit(
        &self,
        deposit_amount_a: Uint128,
        deposit_amount_b: Uint128,
        deposit_amount_c: Uint128,
        swap_amount_a: Uint128,
        swap_amount_b: Uint128,
        swap_amount_c: Uint128,
        pool_token_supply: Uint128,
    ) -> Option<Uint128> {
        // Initial invariant
        let d_0 = self.compute_d(swap_amount_a, swap_amount_b, swap_amount_c)?;
        let new_balances = [
            swap_amount_a.checked_add(deposit_amount_a).unwrap(),
            swap_amount_b.checked_add(deposit_amount_b).unwrap(),
            swap_amount_c.checked_add(deposit_amount_c).unwrap(),
        ];
        // Invariant after change
        let d_1 = self.compute_d(new_balances[0], new_balances[1], new_balances[2])?;
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
        &self,
        swap_in: Uint128,
        //swap_out: Uint128,
        no_swap: Uint128,
        d: Uint256,
    ) -> Option<Uint256> {
        let amp_factor = self.compute_amp_factor()?;
        let ann = amp_factor.checked_mul(N_COINS.into())?; // A * n ** n

        // sum' = prod' = x
        // c =  D ** (n + 1) / (n ** (2 * n) * prod' * A)
        let mut c = d;

        c = c
            .checked_mul(d)
            .unwrap()
            .checked_div(swap_in.checked_mul(N_COINS.into()).unwrap().into())
            .unwrap();

        c = c
            .checked_mul(d)
            .unwrap()
            .checked_div(no_swap.checked_mul(N_COINS.into()).unwrap().into())
            .unwrap();
        c = c
            .checked_mul(d)
            .unwrap()
            .checked_div(ann.checked_mul(N_COINS.into()).unwrap().into())
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
    pub fn compute_y(&self, x: Uint128, no_swap: Uint128, d: Uint256) -> Option<Uint128> {
        let amount = self.compute_y_raw(x, no_swap, d)?;
        Some(Uint128::try_from(amount).unwrap())
    }

    /// Compute SwapResult after an exchange
    #[allow(clippy::unwrap_used)]
    pub fn swap_to(
        &self,
        source_amount: Uint128,
        swap_source_amount: Uint128,
        swap_destination_amount: Uint128,
        unswaped_amount: Uint128,
    ) -> Option<SwapResult> {
        let y = self.compute_y(
            swap_source_amount.checked_add(source_amount).unwrap(),
            unswaped_amount,
            self.compute_d(swap_source_amount, swap_destination_amount, unswaped_amount)
                .unwrap(),
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
}
/*
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::integer_arithmetic,
    clippy::too_many_arguments
)]
mod tests {
    use super::*;
    use crate::stableswap_math::pool_converter::PoolTokenConverter;
    use proptest::prelude::*;
    use rand::Rng;
    use sim::{Model, MODEL_FEE_DENOMINATOR, MODEL_FEE_NUMERATOR};
    use std::cmp;
    use terraswap::U256;

    const RAMP_TICKS: i64 = 100000;

    #[test]
    fn test_ramp_amp_up() {
        let mut rng = rand::thread_rng();
        let initial_amp_factor = 100;
        let target_amp_factor = initial_amp_factor * 2;
        let start_ramp_ts = rng.gen_range(ZERO_TS..=i64::MAX - RAMP_TICKS);
        let stop_ramp_ts = start_ramp_ts + MIN_RAMP_DURATION;
        println!(
            "start_ramp_ts: {}, stop_ramp_ts: {}",
            start_ramp_ts, stop_ramp_ts
        );

        for tick in 0..RAMP_TICKS {
            let current_ts = start_ramp_ts + tick;
            let invariant = StableSwap::new(
                initial_amp_factor,
                target_amp_factor,
                current_ts,
                start_ramp_ts,
                stop_ramp_ts,
            );
            let expected = if tick >= MIN_RAMP_DURATION {
                target_amp_factor
            } else {
                initial_amp_factor + (initial_amp_factor * tick as u64 / MIN_RAMP_DURATION as u64)
            };
            assert_eq!(invariant.compute_amp_factor().unwrap(), expected);
        }
    }

    #[test]
    fn test_ramp_amp_down() {
        let mut rng = rand::thread_rng();
        let initial_amp_factor = 100;
        let target_amp_factor = initial_amp_factor / 10;
        let amp_range = initial_amp_factor - target_amp_factor;
        let start_ramp_ts = rng.gen_range(ZERO_TS..=i64::MAX - RAMP_TICKS);
        let stop_ramp_ts = start_ramp_ts + MIN_RAMP_DURATION;
        println!(
            "start_ramp_ts: {}, stop_ramp_ts: {}",
            start_ramp_ts, stop_ramp_ts
        );

        for tick in 0..RAMP_TICKS {
            let current_ts = start_ramp_ts + tick;
            let invariant = StableSwap::new(
                initial_amp_factor,
                target_amp_factor,
                current_ts,
                start_ramp_ts,
                stop_ramp_ts,
            );
            let expected = if tick >= MIN_RAMP_DURATION {
                target_amp_factor
            } else {
                initial_amp_factor - (amp_range * tick as u64 / MIN_RAMP_DURATION as u64)
            };
            assert_eq!(invariant.compute_amp_factor().unwrap(), expected);
        }
    }

    fn check_d(
        model: &Model,
        amount_a: u128,
        amount_b: u128,
        amount_c: u128,
        current_ts: i64,
        start_ramp_ts: i64,
        stop_ramp_ts: i64,
    ) -> Uint256 {
        let swap = StableSwap {
            initial_amp_factor: model.amp_factor,
            target_amp_factor: model.amp_factor,
            current_ts,
            start_ramp_ts,
            stop_ramp_ts,
        };
        let d = swap
            .compute_d(
                Uint128::new(amount_a),
                Uint128::new(amount_b),
                Uint128::new(amount_c),
            )
            .unwrap();
        assert_eq!(d, Uint256::from(model.sim_d()));
        d
    }

    fn check_y(
        model: &Model,
        swap_in: u128,
        no_swap: u128,
        d: Uint256,
        current_ts: i64,
        start_ramp_ts: i64,
        stop_ramp_ts: i64,
    ) {
        let swap = StableSwap {
            initial_amp_factor: model.amp_factor,
            target_amp_factor: model.amp_factor,
            current_ts,
            start_ramp_ts,
            stop_ramp_ts,
        };
        let y = swap
            .compute_y_raw(Uint128::new(swap_in), Uint128::new(no_swap), d)
            .unwrap();
        assert_eq!(
            Uint128::try_from(y).unwrap().u128(),
            model.sim_y(0, 1, swap_in)
        )
    }

    proptest! {
        #[test]
        fn test_curve_math(
            current_ts in ZERO_TS..i64::MAX,
            amp_factor in MIN_AMP..=MAX_AMP,
            amount_a in 1..MAX_TOKENS_IN.u128(),    // Start at 1 to prevent divide by 0 when computing d
            amount_b in 1..MAX_TOKENS_IN.u128(),    // Start at 1 to prevent divide by 0 when computing d
            amount_c in 1..MAX_TOKENS_IN.u128(),    // Start at 1 to prevent divide by 0 when computing d
        ) {
            let start_ramp_ts = cmp::max(0, current_ts - MIN_RAMP_DURATION);
            let stop_ramp_ts = cmp::min(i64::MAX, current_ts + MIN_RAMP_DURATION);
            let model = Model::new(amp_factor, vec![amount_a, amount_b, amount_c], N_COINS);
            let d = check_d(&model, amount_a, amount_b, amount_c, current_ts, start_ramp_ts, stop_ramp_ts);
            check_y(&model, amount_a, amount_c, d, current_ts, start_ramp_ts, stop_ramp_ts);
        }
    }

    #[test]
    fn test_curve_math_specific() {
        // Specific cases
        let current_ts = ZERO_TS;
        let start_ramp_ts = ZERO_TS;
        let stop_ramp_ts = ZERO_TS;
        let model_no_balance = Model::new(1, vec![0, 0, 0], N_COINS);
        check_d(
            &model_no_balance,
            0,
            0,
            0,
            current_ts,
            start_ramp_ts,
            stop_ramp_ts,
        );

        let amount_a = 1046129065254161082u128;
        let amount_b = 1250710035549196829u128;
        let amount_c = 1111111111111111111u128;
        let model = Model::new(1188, vec![amount_a, amount_b, amount_c], N_COINS);
        let d = check_d(
            &model,
            amount_a,
            amount_b,
            amount_c,
            current_ts,
            start_ramp_ts,
            stop_ramp_ts,
        );
        let amount_x = 2045250484898639148u128;
        check_y(
            &model,
            amount_x,
            amount_c,
            d,
            current_ts,
            start_ramp_ts,
            stop_ramp_ts,
        );

        let amount_a = 862538457714585493u128;
        let amount_b = 492548187909826733u128;
        let amount_c = 777777777777777777u128;
        let model = Model::new(9, vec![amount_a, amount_b, amount_c], N_COINS);
        let d = check_d(
            &model,
            amount_a,
            amount_b,
            amount_c,
            current_ts,
            start_ramp_ts,
            stop_ramp_ts,
        );
        let amount_x = 815577754938955939u128;

        check_y(
            &model,
            amount_x,
            amount_c,
            d,
            current_ts,
            start_ramp_ts,
            stop_ramp_ts,
        );
    }

    #[test]
    fn test_compute_mint_amount_for_deposit() {
        let initial_amp_factor = MIN_AMP;
        let target_amp_factor = MAX_AMP;
        let current_ts = MIN_RAMP_DURATION / 2;
        let start_ramp_ts = ZERO_TS;
        let stop_ramp_ts = MIN_RAMP_DURATION;
        let invariant = StableSwap::new(
            initial_amp_factor,
            target_amp_factor,
            current_ts,
            start_ramp_ts,
            stop_ramp_ts,
        );

        let deposit_amount_a = MAX_TOKENS_IN;
        let deposit_amount_b = MAX_TOKENS_IN;
        let deposit_amount_c = MAX_TOKENS_IN;
        let swap_amount_a = MAX_TOKENS_IN;
        let swap_amount_b = MAX_TOKENS_IN;
        let swap_amount_c = MAX_TOKENS_IN;
        let pool_token_supply = MAX_TOKENS_IN;
        let actual_mint_amount = invariant
            .compute_mint_amount_for_deposit(
                deposit_amount_a,
                deposit_amount_b,
                deposit_amount_c,
                swap_amount_a,
                swap_amount_b,
                swap_amount_c,
                pool_token_supply,
            )
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
            let start_ramp_ts: i64 = rng.gen_range(ZERO_TS..=i64::MAX);
            let stop_ramp_ts: i64 = rng.gen_range(start_ramp_ts..=i64::MAX);
            let current_ts: i64 = rng.gen_range(start_ramp_ts..=stop_ramp_ts);
            println!("testing curve_math_with_random_inputs:");
            println!(
                "current_ts: {}, start_ramp_ts: {}, stop_ramp_ts: {}",
                current_ts, start_ramp_ts, stop_ramp_ts
            );
            println!(
                "amp_factor: {}, amount_a: {}, amount_b: {}, amount_c: {}",
                amp_factor, amount_a, amount_b, amount_c,
            );

            let model = Model::new(amp_factor, vec![amount_a, amount_b, amount_c], N_COINS);
            let d = check_d(
                &model,
                amount_a,
                amount_b,
                amount_c,
                current_ts,
                start_ramp_ts,
                stop_ramp_ts,
            );
            let amount_x = rng.gen_range(0..=amount_a);

            println!("amount_x: {}", amount_x);
            check_y(
                &model,
                amount_x,
                amount_c,
                d,
                current_ts,
                start_ramp_ts,
                stop_ramp_ts,
            );
        }
    }

    fn check_swap(
        initial_amp_factor: u64,
        target_amp_factor: u64,
        current_ts: i64,
        start_ramp_ts: i64,
        stop_ramp_ts: i64,
        source_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
        unswaped_amount: u128,
    ) {
        let swap = StableSwap::new(
            initial_amp_factor,
            target_amp_factor,
            current_ts,
            start_ramp_ts,
            stop_ramp_ts,
        );
        let result = swap
            .swap_to(
                Uint128::new(source_amount),
                Uint128::new(swap_source_amount),
                Uint128::new(swap_destination_amount),
                Uint128::new(unswaped_amount),
            )
            .unwrap();
        let model = Model::new(
            swap.compute_amp_factor().unwrap(),
            vec![swap_source_amount, swap_destination_amount, unswaped_amount],
            N_COINS,
        );

        let expected_amount_swapped = model.sim_exchange(0, 1, source_amount.into());
        let diff = (expected_amount_swapped as i128 - result.amount_swapped.u128() as i128).abs();
        let tolerance = std::cmp::max(1, expected_amount_swapped as i128 / 1_000_000_000);
        assert!(
            diff <= tolerance,
            "result={:?}, expected_amount_swapped={}, amp={}, source_amount={}, swap_source_amount={}, swap_destination_amount={}, diff={}",
            result,
            expected_amount_swapped,
            swap.compute_amp_factor().unwrap(),
            source_amount,
            swap_source_amount,
            swap_destination_amount,
            diff
        );
        assert_eq!(
            result.new_source_amount.u128(),
            swap_source_amount + source_amount
        );
        assert_eq!(
            result.new_destination_amount.u128(),
            swap_destination_amount - result.amount_swapped.u128()
        );
    }

    proptest! {
        #[test]
        fn test_swap_calculation(
            current_ts in ZERO_TS..i64::MAX,
            amp_factor in MIN_AMP..=MAX_AMP,
            source_amount in 0..MAX_TOKENS_IN.u128(),
            swap_source_amount in 0..MAX_TOKENS_IN.u128(),
            swap_destination_amount in 0..MAX_TOKENS_IN.u128(),
            unswaped_amount in 0..MAX_TOKENS_IN.u128(),
        ) {
            let start_ramp_ts = cmp::max(0, current_ts - MIN_RAMP_DURATION);
            let stop_ramp_ts = cmp::min(i64::MAX, current_ts + MIN_RAMP_DURATION);
            check_swap(
                amp_factor,
                amp_factor,
                current_ts,
                start_ramp_ts,
                stop_ramp_ts,
                source_amount,
                swap_source_amount,
                swap_destination_amount,
                unswaped_amount,
            );
        }
    }

    #[ignore]
    #[test]
    fn test_swap_calculation_with_random_inputs() {
        for _ in 0..100 {
            let mut rng = rand::thread_rng();

            let initial_amp_factor: u64 = rng.gen_range(MIN_AMP..=MAX_AMP);
            let target_amp_factor: u64 = rng.gen_range(MIN_AMP..=MAX_AMP);
            let start_ramp_ts: i64 = rng.gen_range(ZERO_TS..=i64::MAX);
            let stop_ramp_ts: i64 = rng.gen_range(start_ramp_ts..=i64::MAX);
            let current_ts: i64 = rng.gen_range(start_ramp_ts..=stop_ramp_ts);
            let source_amount = rng.gen_range(1..=MAX_TOKENS_IN.u128());
            let swap_source_amount = rng.gen_range(1..=MAX_TOKENS_IN.u128());
            let swap_destination_amount = rng.gen_range(1..=MAX_TOKENS_IN.u128());
            let unswaped_amount = rng.gen_range(1..=MAX_TOKENS_IN.u128());

            println!("testing swap_calculation_with_random_inputs:");
            println!(
                "current_ts: {}, start_ramp_ts: {}, stop_ramp_ts: {}",
                current_ts, start_ramp_ts, stop_ramp_ts
            );
            println!(
                "initial_amp_factor: {}, target_amp_factor: {}, source_amount: {}, swap_source_amount: {}, swap_destination_amount: {}, unswaped_amount: {}",
                initial_amp_factor, target_amp_factor, source_amount, swap_source_amount, swap_destination_amount, unswaped_amount
            );

            check_swap(
                initial_amp_factor,
                target_amp_factor,
                current_ts,
                start_ramp_ts,
                stop_ramp_ts,
                source_amount,
                swap_source_amount,
                swap_destination_amount,
                unswaped_amount,
            );
        }
    }

    #[derive(Debug)]

    //TODO  this never does swaps to or from C
    struct SwapTest<'a> {
        pub stable_swap: &'a StableSwap,
        pub swap_reserve_balance_a: Uint128,
        pub swap_reserve_balance_b: Uint128,
        pub swap_reserve_balance_c: Uint128,
        pub user_token_balance_a: Uint128,
        pub user_token_balance_b: Uint128,
        pub user_token_balance_c: Uint128,
    }

    impl SwapTest<'_> {
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
            } = self
                .stable_swap
                .swap_to(
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
            initial_user_token_c_amount in 10_000_000..MAX_TOKENS_IN.u128() >> 16,
        ) {

            let stable_swap = StableSwap {
                initial_amp_factor: amp_factor,
                target_amp_factor: amp_factor,
                current_ts: ZERO_TS,
                start_ramp_ts: ZERO_TS,
                stop_ramp_ts: ZERO_TS
            };
            let mut t = SwapTest { stable_swap: &stable_swap, swap_reserve_balance_a: MAX_TOKENS_IN, swap_reserve_balance_b: MAX_TOKENS_IN,
                swap_reserve_balance_c: MAX_TOKENS_IN,
                user_token_balance_a: Uint128::new(initial_user_token_a_amount),
                user_token_balance_b:Uint128::new(initial_user_token_b_amount),
                user_token_balance_c: Uint128::new(initial_user_token_c_amount) };

            const ITERATIONS: u64 = 100;
            const SHRINK_MULTIPLIER: u64= 10;

            for i in 0..ITERATIONS {
                let before_balance_a = t.user_token_balance_a;
                let before_balance_b = t.user_token_balance_b;
                let swap_amount = before_balance_a / Uint128::from((i + 1) * SHRINK_MULTIPLIER);
                t.swap_a_to_b(swap_amount);
                let after_balance = t.user_token_balance_a + t.user_token_balance_b;

                assert!(before_balance_a + before_balance_b >= after_balance, "before_a: {}, before_b: {}, after_a: {}, after_b: {}, swap: {:?}", before_balance_a, before_balance_b, t.user_token_balance_a, t.user_token_balance_b, stable_swap);
            }

            for i in 0..ITERATIONS {
                let before_balance_a = t.user_token_balance_a;
                let before_balance_b = t.user_token_balance_b;
                let swap_amount = before_balance_a / Uint128::from((i + 1) * SHRINK_MULTIPLIER);
                t.swap_a_to_b(swap_amount);
                let after_balance = t.user_token_balance_a + t.user_token_balance_b;

                assert!(before_balance_a + before_balance_b >= after_balance, "before_a: {}, before_b: {}, after_a: {}, after_b: {}, swap: {:?}", before_balance_a, before_balance_b, t.user_token_balance_a, t.user_token_balance_b, stable_swap);
            }
        }
    }

    #[test]
    fn test_swaps_does_not_result_in_more_tokens_specific_one() {
        const AMP_FACTOR: u64 = 324449;
        const INITIAL_SWAP_RESERVE_AMOUNT: Uint128 = Uint128::new(100_000_000_000u128);
        const INITIAL_USER_TOKEN_AMOUNT: Uint128 = Uint128::new(10_000_000_000u128);

        let stable_swap = StableSwap {
            initial_amp_factor: AMP_FACTOR,
            target_amp_factor: AMP_FACTOR,
            current_ts: ZERO_TS,
            start_ramp_ts: ZERO_TS,
            stop_ramp_ts: ZERO_TS,
        };

        let mut t = SwapTest {
            stable_swap: &stable_swap,
            swap_reserve_balance_a: INITIAL_SWAP_RESERVE_AMOUNT,
            swap_reserve_balance_b: INITIAL_SWAP_RESERVE_AMOUNT,
            swap_reserve_balance_c: INITIAL_SWAP_RESERVE_AMOUNT,
            user_token_balance_a: INITIAL_USER_TOKEN_AMOUNT,
            user_token_balance_b: INITIAL_USER_TOKEN_AMOUNT,
            user_token_balance_c: INITIAL_USER_TOKEN_AMOUNT,
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

        let stable_swap = StableSwap {
            initial_amp_factor: AMP_FACTOR,
            target_amp_factor: AMP_FACTOR,
            current_ts: ZERO_TS,
            start_ramp_ts: ZERO_TS,
            stop_ramp_ts: ZERO_TS,
        };

        let mut t = SwapTest {
            stable_swap: &stable_swap,
            swap_reserve_balance_a: INITIAL_SWAP_RESERVE_AMOUNT,
            swap_reserve_balance_b: INITIAL_SWAP_RESERVE_AMOUNT,
            swap_reserve_balance_c: INITIAL_SWAP_RESERVE_AMOUNT,
            user_token_balance_a: INITIAL_USER_TOKEN_AMOUNT,
            user_token_balance_b: INITIAL_USER_TOKEN_AMOUNT,
            user_token_balance_c: INITIAL_USER_TOKEN_AMOUNT,
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

        let stable_swap = StableSwap {
            initial_amp_factor: AMP_FACTOR,
            target_amp_factor: AMP_FACTOR,
            current_ts: ZERO_TS,
            start_ramp_ts: ZERO_TS,
            stop_ramp_ts: ZERO_TS,
        };

        let mut t = SwapTest {
            stable_swap: &stable_swap,
            swap_reserve_balance_a: INITIAL_SWAP_RESERVE_AMOUNT,
            swap_reserve_balance_b: INITIAL_SWAP_RESERVE_AMOUNT,
            swap_reserve_balance_c: INITIAL_SWAP_RESERVE_AMOUNT,
            user_token_balance_a: INITIAL_USER_TOKEN_AMOUNT,
            user_token_balance_b: INITIAL_USER_TOKEN_AMOUNT,
            user_token_balance_c: INITIAL_USER_TOKEN_AMOUNT,
        };

        t.swap_b_to_a(Uint128::from(65535u128));
        t.swap_b_to_a(Uint128::from(6133503u128));
        t.swap_a_to_b(Uint128::from(65535u128));
        assert!(
            t.user_token_balance_a + t.user_token_balance_b
                <= INITIAL_USER_TOKEN_AMOUNT * Uint128::from(2u8)
        );
    }

    fn check_withdraw_one(
        initial_amp_factor: u64,
        target_amp_factor: u64,
        current_ts: i64,
        start_ramp_ts: i64,
        stop_ramp_ts: i64,
        pool_token_amount: Uint128,
        pool_token_supply: Uint128,
        swap_base_amount: Uint128,
        swap_quote_amount: Uint128,
        swap_unswapped_amount: Uint128,
    ) {
        let swap = StableSwap::new(
            initial_amp_factor,
            target_amp_factor,
            current_ts,
            start_ramp_ts,
            stop_ramp_ts,
        );

        let result = swap
            .compute_withdraw_one(
                pool_token_amount,
                pool_token_supply,
                swap_base_amount,
                swap_quote_amount,
                swap_unswapped_amount,
            )
            .unwrap();
        let model = Model::new_with_pool_tokens(
            swap.compute_amp_factor().unwrap(),
            vec![swap_base_amount.u128(), swap_quote_amount.u128()],
            N_COINS,
            pool_token_supply.u128(),
        );
        assert_eq!(
            result.0.u128(),
            model
                .sim_calc_withdraw_one_coin(pool_token_amount.u128(), 0)
                .0
        );
        assert_eq!(
            result.1.u128(),
            model
                .sim_calc_withdraw_one_coin(pool_token_amount.u128(), 0)
                .1
        );
    }

    proptest! {
        #[ignore]
        #[test]
        fn test_compute_withdraw_one(
            current_ts in ZERO_TS..i64::MAX,
            amp_factor in MIN_AMP..=MAX_AMP,
            pool_token_amount in 1..MAX_TOKENS_IN.u128() / 2,
            swap_base_amount in 1..MAX_TOKENS_IN.u128() / 2,
            swap_quote_amount in 1..MAX_TOKENS_IN.u128() / 2,
            swap_unswapped_amount in 1..MAX_TOKENS_IN.u128() / 2,
        ) {
            let pool_token_supply = MAX_TOKENS_IN;
            let start_ramp_ts = cmp::max(0, current_ts - MIN_RAMP_DURATION);
            let stop_ramp_ts = cmp::min(i64::MAX, current_ts + MIN_RAMP_DURATION);
            check_withdraw_one(
                amp_factor,
                amp_factor,
                current_ts,
                start_ramp_ts,
                stop_ramp_ts,
                Uint128::new(pool_token_amount),
                pool_token_supply,
                Uint128::new(swap_base_amount),
                Uint128::new(swap_quote_amount),
                Uint128::new(swap_unswapped_amount),
            );
        }
    }
    /*  TODO commented out proptests
       #[ignore]
       #[test]
       fn test_compute_withdraw_one_with_random_inputs() {
           for _ in 0..100 {
               let mut rng = rand::thread_rng();

               let initial_amp_factor: u64 = rng.gen_range(MIN_AMP..=MAX_AMP);
               let target_amp_factor: u64 = rng.gen_range(MIN_AMP..=MAX_AMP);
               let start_ramp_ts: i64 = rng.gen_range(ZERO_TS..=i64::MAX);
               let stop_ramp_ts: i64 = rng.gen_range(start_ramp_ts..=i64::MAX);
               let current_ts: i64 = rng.gen_range(start_ramp_ts..=stop_ramp_ts);
               let swap_base_amount: Uint128 = rng.gen_range(1..=MAX_TOKENS_IN);
               let swap_quote_amount: Uint128 = rng.gen_range(1..=MAX_TOKENS_IN);
               let swap_unswapped_amount: Uint128 = rng.gen_range(1..=MAX_TOKENS_IN);
               let pool_token_supply = swap_base_amount + swap_quote_amount + swap_unswapped_amount;
               let pool_token_amount: Uint128 = rng.gen_range(1..=pool_token_supply);
               println!("testing compute_withdraw_one_with_random_inputs:");
               println!(
                   "current_ts: {}, start_ramp_ts: {}, stop_ramp_ts: {}",
                   current_ts, start_ramp_ts, stop_ramp_ts
               );
               println!(
                   "initial_amp_factor: {}, target_amp_factor: {}, swap_base_amount: {}, swap_quote_amount: {}, pool_token_amount: {}, pool_token_supply: {}",
                   initial_amp_factor, target_amp_factor,  swap_base_amount, swap_quote_amount, pool_token_amount, pool_token_supply
               );

               check_withdraw_one(
                   initial_amp_factor,
                   target_amp_factor,
                   current_ts,
                   start_ramp_ts,
                   stop_ramp_ts,
                   pool_token_amount,
                   pool_token_supply,
                   swap_base_amount,
                   swap_quote_amount,
                   swap_unswapped_amount,
               );
           }
       }

       proptest! {
           #[test]
           fn test_virtual_price_does_not_decrease_from_deposit(
               current_ts in ZERO_TS..i64::MAX,
               amp_factor in MIN_AMP..=MAX_AMP,
               deposit_amount_a in 0..MAX_TOKENS_IN >> 2,
               deposit_amount_b in 0..MAX_TOKENS_IN >> 2,
               deposit_amount_c in 0..MAX_TOKENS_IN >> 2,
               swap_token_a_amount in 0..MAX_TOKENS_IN,
               swap_token_b_amount in 0..MAX_TOKENS_IN,
               swap_token_c_amount in 0..MAX_TOKENS_IN,
               pool_token_supply in 0..MAX_TOKENS_IN,
           ) {
               let deposit_amount_a = deposit_amount_a;
               let deposit_amount_b = deposit_amount_b;
               let deposit_amount_c = deposit_amount_c;
               let swap_token_a_amount = swap_token_a_amount;
               let swap_token_b_amount = swap_token_b_amount;
               let swap_token_c_amount = swap_token_c_amount;
               let pool_token_supply = pool_token_supply;

               let start_ramp_ts = cmp::max(0, current_ts - MIN_RAMP_DURATION);
               let stop_ramp_ts = cmp::min(i64::MAX, current_ts + MIN_RAMP_DURATION);
               let invariant = StableSwap::new(amp_factor, amp_factor, current_ts, start_ramp_ts, stop_ramp_ts);
               let d0 = invariant.compute_d(swap_token_a_amount, swap_token_b_amount, swap_token_c_amount).unwrap();

               let mint_amount = invariant.compute_mint_amount_for_deposit(
                       deposit_amount_a,
                       deposit_amount_b,
                       deposit_amount_c,
                       swap_token_a_amount,
                       swap_token_b_amount,
                       swap_token_c_amount,
                       pool_token_supply,
                   );
               prop_assume!(mint_amount.is_some());

               let new_swap_token_a_amount = swap_token_a_amount + deposit_amount_a;
               let new_swap_token_b_amount = swap_token_b_amount + deposit_amount_b;
               let new_swap_token_c_amount = swap_token_c_amount + deposit_amount_c;
               let new_pool_token_supply = pool_token_supply + mint_amount.unwrap();
               let d1 = invariant.compute_d(new_swap_token_a_amount, new_swap_token_b_amount, new_swap_token_c_amount).unwrap();

               assert!(d0 < d1);
               assert!(d0 / pool_token_supply <= d1 / new_pool_token_supply);
           }
       }

       proptest! {
           #[test]
           fn test_virtual_price_does_not_decrease_from_swap(
               current_ts in ZERO_TS..i64::MAX,
               amp_factor in MIN_AMP..=MAX_AMP,
               source_token_amount in 0..MAX_TOKENS_IN,
               swap_source_amount in 0..MAX_TOKENS_IN,
               swap_destination_amount in 0..MAX_TOKENS_IN,
               unswapped_amount in 0..MAX_TOKENS_IN,
           ) {
               let source_token_amount = source_token_amount;
               let swap_source_amount = swap_source_amount;
               let swap_destination_amount = swap_destination_amount;
               let unswapped_amount = unswapped_amount;

               let start_ramp_ts = cmp::max(0, current_ts - MIN_RAMP_DURATION);
               let stop_ramp_ts = cmp::min(i64::MAX, current_ts + MIN_RAMP_DURATION);
               let invariant = StableSwap::new(amp_factor, amp_factor, current_ts, start_ramp_ts, stop_ramp_ts);
               let d0 = invariant.compute_d(swap_source_amount, swap_destination_amount, unswapped_amount).unwrap();

               let swap_result = invariant.swap_to(source_token_amount, swap_source_amount, swap_destination_amount, unswapped_amount);
               prop_assume!(swap_result.is_some());

               let swap_result = swap_result.unwrap();
               let d1 = invariant.compute_d(swap_result.new_source_amount, swap_result.new_destination_amount, unswapped_amount).unwrap();

               assert!(d0 <= d1);  // Pool token supply not changed on swaps
           }
       }

       proptest! {
           #[test]
           fn test_virtual_price_does_not_decrease_from_withdraw(
               current_ts in ZERO_TS..i64::MAX,
               amp_factor in MIN_AMP..=MAX_AMP,
               (pool_token_supply, pool_token_amount) in total_and_intermediate(),
               swap_token_a_amount in 0..MAX_TOKENS_IN,
               swap_token_b_amount in 0..MAX_TOKENS_IN,
               unswapped_amount in 0..MAX_TOKENS_IN,
           ) {
               let swap_token_a_amount = swap_token_a_amount;
               let swap_token_b_amount = swap_token_b_amount;
               let pool_token_amount = pool_token_amount;
               let pool_token_supply = pool_token_supply;
               let unswapped_amount = unswapped_amount;

               let start_ramp_ts = cmp::max(0, current_ts - MIN_RAMP_DURATION);
               let stop_ramp_ts = cmp::min(i64::MAX, current_ts + MIN_RAMP_DURATION);
               let invariant = StableSwap::new(amp_factor, amp_factor, current_ts, start_ramp_ts, stop_ramp_ts);
               let d0 = invariant.compute_d(swap_token_a_amount, swap_token_b_amount, unswapped_amount).unwrap();

               let converter = PoolTokenConverter {
                   supply: pool_token_supply,
                   token_a: swap_token_a_amount,
                   token_b: swap_token_b_amount,
               };

               // Make sure we will get at least one trading token out for each
               // side, otherwise the calculation fails
               prop_assume!((pool_token_amount as u128) * (swap_token_a_amount as u128) / (pool_token_supply as u128) >= 1);
               prop_assume!((pool_token_amount as u128) * (swap_token_b_amount as u128) / (pool_token_supply as u128) >= 1);

               let (withdraw_amount_a, _, _) = converter.token_a_rate(pool_token_amount).unwrap();
               let (withdraw_amount_b, _, _) = converter.token_b_rate(pool_token_amount).unwrap();

               let new_swap_token_a_amount = swap_token_a_amount - withdraw_amount_a;
               let new_swap_token_b_amount = swap_token_b_amount - withdraw_amount_b;
               let d1 = invariant.compute_d(new_swap_token_a_amount, new_swap_token_b_amount, unswapped_amount).unwrap();
               let new_pool_token_supply = pool_token_supply - pool_token_amount;

               assert!(d0 / pool_token_supply <= d1 / new_pool_token_supply);
           }
       }

       proptest! {
           #[test]
           fn test_virtual_price_does_not_decrease_from_withdraw_one(
               current_ts in ZERO_TS..i64::MAX,
               amp_factor in MIN_AMP..MAX_AMP,
               (pool_token_supply, pool_token_amount) in total_and_intermediate(),
               base_token_amount in 0..MAX_TOKENS_IN,
               quote_token_amount in 0..MAX_TOKENS_IN,
               unswapped_amount in 0..MAX_TOKENS_IN,
           ) {
               let base_token_amount = base_token_amount;
               let quote_token_amount = quote_token_amount;
               let pool_token_amount = pool_token_amount;
               let pool_token_supply = pool_token_supply;
               let unswapped_amount = unswapped_amount;

               let start_ramp_ts = cmp::max(0, current_ts - MIN_RAMP_DURATION);
               let stop_ramp_ts = cmp::min(i64::MAX, current_ts + MIN_RAMP_DURATION);
               let invariant = StableSwap::new(amp_factor, amp_factor, current_ts, start_ramp_ts, stop_ramp_ts);
               let d0 = invariant.compute_d(base_token_amount, quote_token_amount, unswapped_amount).unwrap();

               prop_assume!(Uint256::from(pool_token_amount) * Uint256::from(base_token_amount) / Uint256::from(pool_token_supply) >= Uint256::from(1));
               let (withdraw_amount, _) = invariant.compute_withdraw_one(pool_token_amount, pool_token_supply, base_token_amount, quote_token_amount, unswapped_amount).unwrap();

               let new_base_token_amount = base_token_amount - withdraw_amount;
               let d1 = invariant.compute_d(new_base_token_amount, quote_token_amount, unswapped_amount).unwrap();
               let new_pool_token_supply = pool_token_supply - pool_token_amount;

               assert!(d0 / pool_token_supply <= d1 / new_pool_token_supply);
           }
       }

       prop_compose! {
           pub fn total_and_intermediate()(total in 1..MAX_TOKENS_IN.u128())
                           (intermediate in 1..total, total in Just(total))
                           -> (Uint128, Uint128) {
              (total, intermediate)
          }
       }

    */
}
*/
