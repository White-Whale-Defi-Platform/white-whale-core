#[cfg(test)]
mod tests {
    use cosmwasm_std::{Decimal, Decimal256, Uint128};
    use terraswap::{asset::PairType, pair::PoolFee};
    use white_whale::fee::Fee;

    use crate::{
        helpers::{calculate_stableswap_y, compute_swap, StableSwapDirection, SwapComputation},
        math::Decimal256Helper,
    };

    #[test]
    fn does_calculate_y_value() {
        let amp = 100;

        let offer_pool = Decimal256::decimal_with_precision(100_000_000_000u128, 6).unwrap();
        let ask_pool = Decimal256::decimal_with_precision(100_000_000_000u128, 6).unwrap();
        let offer_amount = Decimal256::decimal_with_precision(100_000000u128, 6).unwrap();

        let y = calculate_stableswap_y(
            offer_pool,
            ask_pool,
            offer_amount,
            &amp,
            6,
            6,
            StableSwapDirection::Simulate,
        )
        .unwrap();
        assert_eq!(y, Uint128::new(99_900_000_990));

        let offer_pool = Decimal256::decimal_with_precision(1_010_000u128, 6).unwrap();
        let ask_pool = Decimal256::decimal_with_precision(990_050u128, 6).unwrap();
        let offer_amount = Decimal256::decimal_with_precision(10_000u128, 6).unwrap();

        let y = calculate_stableswap_y(
            offer_pool,
            ask_pool,
            offer_amount,
            &amp,
            6,
            6,
            StableSwapDirection::Simulate,
        )
        .unwrap();
        assert_eq!(y, Uint128::new(980_053));
    }

    #[test]
    fn does_stableswap_correctly() {
        let asset_pool_amount = Uint128::from(990_050u128);
        let collateral_pool_amount = Uint128::from(1_010_000u128);
        let offer_amount = Uint128::from(10_000u128);

        let swap_result = compute_swap(
            collateral_pool_amount,
            asset_pool_amount,
            offer_amount,
            PoolFee {
                protocol_fee: Fee {
                    share: Decimal::from_ratio(1u128, 400u128),
                },
                swap_fee: Fee {
                    share: Decimal::from_ratio(1u128, 400u128),
                },
                burn_fee: Fee {
                    share: Decimal::zero(),
                },
            },
            &PairType::StableSwap { amp: 100 },
            6,
            6,
        )
        .unwrap();

        assert_eq!(
            swap_result,
            SwapComputation {
                protocol_fee_amount: Uint128::new(24),
                swap_fee_amount: Uint128::new(24),
                return_amount: Uint128::new(9_949),
                spread_amount: Uint128::new(3),
                burn_fee_amount: Uint128::zero()
            }
        );
    }
}
