use std::vec;

use cosmwasm_std::{coin, Coin, Decimal, Uint128};

use white_whale_std::fee::{Fee, PoolFee};
use white_whale_std::pool_manager::SwapRoute;

use crate::tests::suite::TestingSuite;

#[test]
fn test_fill_rewards_from_pool_manager() {
    let mut suite = TestingSuite::default();
    let creator = suite.senders[0].clone();

    let asset_denoms = vec!["uwhale".to_string(), "uusdc".to_string()];

    // Default Pool fees white_whale_std::pool_network::pair::PoolFee
    // Protocol fee is 0.01% and swap fee is 0.02% and burn fee is 0%
    #[cfg(not(feature = "osmosis"))]
    let pool_fees = PoolFee {
        protocol_fee: Fee {
            share: Decimal::from_ratio(1u128, 100u128),
        },
        swap_fee: Fee {
            share: Decimal::from_ratio(1u128, 100u128),
        },
        burn_fee: Fee {
            share: Decimal::zero(),
        },
        extra_fees: vec![],
    };

    #[cfg(feature = "osmosis")]
        let pool_fees = PoolFee {
        protocol_fee: Fee {
            share: Decimal::from_ratio(1u128, 100u128),
        },
        swap_fee: Fee {
            share: Decimal::from_ratio(1u128, 100u128),
        },
        burn_fee: Fee {
            share: Decimal::zero(),
        },
        extra_fees: vec![],
        osmosis_fee: Fee {
            share: Decimal::permille(1),
        },
    };

    suite
        .instantiate_default()
        .fast_forward(90_000)
        .create_new_epoch()
        .create_pair(
            creator.clone(),
            asset_denoms.clone(),
            pool_fees.clone(),
            white_whale_std::pool_manager::PoolType::ConstantProduct,
            Some("whale-uusdc".to_string()),
            vec![coin(1000, "uwhale")],
            |result| {
                result.unwrap();
            },
        );

    // Lets try to add liquidity
    suite.provide_liquidity(
        creator.clone(),
        "whale-uusdc".to_string(),
        vec![
            Coin {
                denom: "uwhale".to_string(),
                amount: Uint128::from(1_000_000_000u128),
            },
            Coin {
                denom: "uusdc".to_string(),
                amount: Uint128::from(1_000_000_000u128),
            },
        ],
        |result| {
            result.unwrap();
        },
    );

    // Lets try to add a swap route
    let swap_route_1 = SwapRoute {
        offer_asset_denom: "uusdc".to_string(),
        ask_asset_denom: "uwhale".to_string(),
        swap_operations: vec![white_whale_std::pool_manager::SwapOperation::WhaleSwap {
            token_in_denom: "uusdc".to_string(),
            token_out_denom: "uwhale".to_string(),
            pool_identifier: "whale-uusdc".to_string(),
        }],
    };

    suite
        .add_swap_routes(creator.clone(), vec![swap_route_1], |res| {
            res.unwrap();
        })
        .swap(
            creator.clone(),
            coin(1_000u128, "uusdc"),
            "uwhale".to_string(),
            None,
            None,
            None,
            "whale-uusdc".to_string(),
            vec![Coin {
                denom: "uusdc".to_string(),
                amount: Uint128::from(1_000u128),
            }],
            |result| {
                result.unwrap();
            },
        );

    // Get balance of the bonding manager it should have received fees from the swap
    suite
        .query_balance(
            "uwhale".to_string(),
            suite.bonding_manager_addr.clone(),
            |res| {
                // 1_000u128 - 9u128 swap_fee - 9u128 protocol_fee where protocol_fee and swap_fee are 1% of the swap amount
                // + 1_000u128 uwhale pool creation fee
                assert_eq!(res, Uint128::from(1009u128));
            },
        )
        .create_pair(
            creator.clone(),
            asset_denoms.clone(),
            pool_fees.clone(),
            white_whale_std::pool_manager::PoolType::ConstantProduct,
            Some("whale-uusdc-second".to_string()),
            vec![coin(1000, "uwhale")],
            |result| {
                result.unwrap();
            },
        );

    // Get balance of the bonding manager again it should have the pool creation fee
    suite.query_balance(
        "uwhale".to_string(),
        suite.bonding_manager_addr.clone(),
        |res| {
            assert_eq!(res, Uint128::from(2009u128));
        },
    );

    // create another pair to collect another fee
    suite.create_pair(
        creator.clone(),
        asset_denoms,
        pool_fees,
        white_whale_std::pool_manager::PoolType::ConstantProduct,
        Some("whale-uusdc-third".to_string()),
        vec![coin(1000, "uwhale")],
        |result| {
            result.unwrap();
        },
    );
    // Verify the fee has been collected
    suite.query_balance(
        "uwhale".to_string(),
        suite.bonding_manager_addr.clone(),
        |res| {
            assert_eq!(res, Uint128::from(3009u128));
        },
    );

    suite.fill_rewards(
        creator.clone(),
        vec![coin(
            1000,
            "factory/contract2/uwhale-uusdc.pool.whale-uusdc.uLP",
        )],
        |res| {
            res.unwrap();
        },
    );

    let bonding_manager_addr = suite.bonding_manager_addr.clone();
    let bonding_manager_balances = suite
        .app
        .wrap()
        .query_all_balances(bonding_manager_addr.clone())
        .unwrap();
    assert_eq!(bonding_manager_balances.len(), 1);
    // 4998 = 4989 (3009 + 1980 (from the lp withdrawal swap)) + 9 (fee from the lp withdrawal swap)
    assert_eq!(
        bonding_manager_balances[0].amount,
        Uint128::from(4989u128 + 9u128)
    );

    // send some random asset that doesn't have swap routes
    suite.fill_rewards(
        creator.clone(),
        vec![coin(1000, "non_whitelisted_asset")],
        |res| {
            res.unwrap();
        },
    );

    let bonding_manager_addr = suite.bonding_manager_addr.clone();
    let bonding_manager_balances = suite
        .app
        .wrap()
        .query_all_balances(bonding_manager_addr.clone())
        .unwrap();
    assert_eq!(bonding_manager_balances.len(), 2);
    assert_eq!(
        bonding_manager_balances,
        vec![
            // wasn't swapped
            Coin {
                denom: "non_whitelisted_asset".to_string(),
                amount: Uint128::from(1000u128),
            },
            Coin {
                denom: "uwhale".to_string(),
                amount: Uint128::from(4998u128),
            },
        ]
    );
}

#[test]
fn test_fill_random_lp_token() {
    let mut suite = TestingSuite::default();
    let creator = suite.senders[0].clone();

    suite
        .instantiate_default()
        .fast_forward(90_000)
        .create_new_epoch();

    // Get balance of the bonding manager it should have received fees from the swap
    suite.query_balance(
        "uwhale".to_string(),
        suite.bonding_manager_addr.clone(),
        |res| {
            assert_eq!(res, Uint128::zero());
        },
    );

    suite.fill_rewards(creator.clone(), vec![coin(5_000, "uwhale")], |res| {
        res.unwrap();
    });

    suite.query_balance(
        "uwhale".to_string(),
        suite.bonding_manager_addr.clone(),
        |res| {
            assert_eq!(res, Uint128::new(5_000));
        },
    );

    // this token doesn't exist in the pool manager. It shouldn't be swapped
    suite.fill_rewards(
        creator.clone(),
        vec![coin(
            1000,
            "factory/contract100/uluna-uwhale.pool.random_identifier.uLP",
        )],
        |res| {
            res.unwrap();
        },
    );

    suite.query_balance(
        "uwhale".to_string(),
        suite.bonding_manager_addr.clone(),
        |res| {
            assert_eq!(res, Uint128::new(5_000));
        },
    );
    suite.query_balance(
        "factory/contract100/uluna-uwhale.pool.random_identifier.uLP".to_string(),
        suite.bonding_manager_addr.clone(),
        |res| {
            assert_eq!(res, Uint128::new(1_000));
        },
    );
}
