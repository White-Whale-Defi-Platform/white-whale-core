use std::vec;

use cosmwasm_std::{coin, Coin, Decimal, Uint128};
use white_whale_std::fee::{Fee, PoolFee};
use white_whale_std::pool_manager::SwapRoute;
use white_whale_std::pool_network::asset::MINIMUM_LIQUIDITY_AMOUNT;

use crate::tests::robot::TestingRobot;
use crate::tests::test_helpers;

#[test]
fn test_fill_rewards_from_pool_manager() {
    let mut robot = TestingRobot::default();
    let creator = robot.sender.clone();
    let epochs = test_helpers::get_epochs();

    let asset_infos = vec!["uwhale".to_string(), "uusdc".to_string()];

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

    robot
        .instantiate_default()
        .add_epochs_to_state(epochs)
        .create_pair(
            creator.clone(),
            asset_infos.clone(),
            pool_fees.clone(),
            white_whale_std::pool_network::asset::PairType::ConstantProduct,
            Some("whale-uusdc".to_string()),
            vec![coin(1000, "uwhale")],
            |result| {
                result.unwrap();
            },
        );

    // Lets try to add liquidity
    robot.provide_liquidity(
        creator.clone(),
        "whale-uusdc".to_string(),
        vec![
            Coin {
                denom: "uwhale".to_string(),
                amount: Uint128::from(1000000000u128),
            },
            Coin {
                denom: "uusdc".to_string(),
                amount: Uint128::from(1000000000u128),
            },
        ],
        |result| {
            println!("{:?}", result.as_ref().unwrap());
            // Ensure we got 999_000 in the response which is 1mil less the initial liquidity amount
            assert!(result.unwrap().events.iter().any(|event| {
                event.attributes.iter().any(|attr| {
                    attr.key == "share"
                        && attr.value
                            == (Uint128::from(1000000000u128) - MINIMUM_LIQUIDITY_AMOUNT)
                                .to_string()
                })
            }));
        },
    );

    println!("{:?}", robot.app.wrap().query_all_balances(creator.clone()));

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
    robot.add_swap_routes(creator.clone(), vec![swap_route_1], |res| {
        println!("{:?}", res.unwrap());
    });

    robot.swap(
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
    robot.query_balance(
        "uwhale".to_string(),
        robot.bonding_manager_addr.clone(),
        |res| {
            // 1_000u128 - 9u128 swap_fee - 9u128 protocol_fee where protocol_fee and swap_fee are 1% of the swap amount
            // + 1_000u128 uwhale pool creation fee
            assert_eq!(res, Uint128::from(1009u128));
        },
    );

    robot.create_pair(
        creator.clone(),
        asset_infos.clone(),
        pool_fees.clone(),
        white_whale_std::pool_network::asset::PairType::ConstantProduct,
        Some("whale-uusdc-second".to_string()),
        vec![coin(1000, "uwhale")],
        |result| {
            result.unwrap();
        },
    );

    // Get balance of the bonding manager again it should have the pool creation fee
    robot.query_balance(
        "uwhale".to_string(),
        robot.bonding_manager_addr.clone(),
        |res| {
            assert_eq!(res, Uint128::from(2009u128));
        },
    );

    // create another pair to collect another fee
    robot.create_pair(
        creator.clone(),
        asset_infos,
        pool_fees,
        white_whale_std::pool_network::asset::PairType::ConstantProduct,
        Some("whale-uusdc-third".to_string()),
        vec![coin(1000, "uwhale")],
        |result| {
            result.unwrap();
        },
    );
    // Verify the fee has been collected
    robot.query_balance(
        "uwhale".to_string(),
        robot.bonding_manager_addr.clone(),
        |res| {
            assert_eq!(res, Uint128::from(3009u128));
        },
    );

    robot.fill_rewards_lp(
        creator.clone(),
        vec![coin(
            1000,
            "factory/contract2/uwhale-uusdc.pool.whale-uusdc.uLP",
        )],
        |res| {
            println!("{:?}", res.unwrap());
        },
    );
}
