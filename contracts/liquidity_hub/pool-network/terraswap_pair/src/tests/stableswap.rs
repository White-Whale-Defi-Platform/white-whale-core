#[cfg(test)]
mod tests {
    use cosmwasm_std::{Decimal, Decimal256, Uint128};
    use pool_network::{asset::PairType, pair::PoolFee};
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

    #[test]
    fn does_stableswap_correctly_with_smaller_asset_pool() {
        // test when asset_pool is smaller than collateral_pool

        let asset_pool_amount = Uint128::from(4_607_500_000_000u128);
        let collateral_pool_amount = Uint128::from(4_602_500_763_431u128);
        let offer_amount = Uint128::from(1_000_000u128);

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
                protocol_fee_amount: Uint128::new(2500),
                swap_fee_amount: Uint128::new(2500),
                return_amount: Uint128::new(995_011),
                spread_amount: Uint128::new(0),
                burn_fee_amount: Uint128::zero()
            }
        );
    }

    #[test]
    #[allow(clippy::inconsistent_digit_grouping)]
    fn does_stableswap_with_different_precisions() {
        let mut asset_pool_amount = Uint128::from(1000000_000000000000000000u128);
        let mut collateral_pool_amount = Uint128::from(1000000_000000u128);
        let mut offer_amount = Uint128::from(1000_000000u128);

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
            18,
        )
        .unwrap();
        assert_eq!(
            swap_result,
            SwapComputation {
                protocol_fee_amount: Uint128::new(2_499975247745560004),
                swap_fee_amount: Uint128::new(2_499975247745560004),
                return_amount: Uint128::new(994_990148602732881842),
                spread_amount: Uint128::new(9900901775998150),
                burn_fee_amount: Uint128::zero()
            }
        );

        // apply the changes of the swap
        asset_pool_amount -= swap_result.return_amount
            + swap_result.spread_amount
            + swap_result.protocol_fee_amount
            + swap_result.swap_fee_amount
            + swap_result.burn_fee_amount;
        collateral_pool_amount += offer_amount;

        // do a new offer, in the opposite direction of 1k this time
        offer_amount = Uint128::new(5000_000000000000000000u128);
        let swap_result = compute_swap(
            asset_pool_amount,
            collateral_pool_amount,
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
            18,
            6,
        )
        .unwrap();
        assert_eq!(
            swap_result,
            SwapComputation {
                protocol_fee_amount: Uint128::new(12_499628),
                swap_fee_amount: Uint128::new(12_499628),
                return_amount: Uint128::new(4974_852233),
                spread_amount: Uint128::new(148511),
                burn_fee_amount: Uint128::zero()
            }
        );
    }

    #[test]
    #[allow(clippy::inconsistent_digit_grouping)]
    fn does_stableswap_with_18_18_precision() {
        let mut asset_pool_amount = Uint128::from(1000000_000000000000000000u128);
        let mut collateral_pool_amount = Uint128::from(1000000_000000000000000000u128);
        let mut offer_amount = Uint128::from(1000_000000000000000000u128);

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
            &PairType::StableSwap { amp: 50 },
            18,
            18,
        )
        .unwrap();
        assert_eq!(
            swap_result,
            SwapComputation {
                protocol_fee_amount: Uint128::new(2_499950981306193408),
                swap_fee_amount: Uint128::new(2_499950981306193408),
                return_amount: Uint128::new(994_980490559864976477),
                spread_amount: Uint128::new(19607477522636707),
                burn_fee_amount: Uint128::zero()
            }
        );

        // apply the changes of the swap
        asset_pool_amount -= swap_result.return_amount
            + swap_result.spread_amount
            + swap_result.protocol_fee_amount
            + swap_result.swap_fee_amount
            + swap_result.burn_fee_amount;
        collateral_pool_amount += offer_amount;

        // do a new offer, in the opposite direction of 5k this time
        offer_amount = Uint128::new(5000_000000000000000000u128);
        let swap_result = compute_swap(
            asset_pool_amount,
            collateral_pool_amount,
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
            &PairType::StableSwap { amp: 50 },
            18,
            18,
        )
        .unwrap();
        assert_eq!(
            swap_result,
            SwapComputation {
                protocol_fee_amount: Uint128::new(12_499264751528814624),
                swap_fee_amount: Uint128::new(12_499264751528814624),
                return_amount: Uint128::new(4974_707371108468220566),
                spread_amount: Uint128::new(294099388474150186),
                burn_fee_amount: Uint128::zero()
            }
        );
    }
}
