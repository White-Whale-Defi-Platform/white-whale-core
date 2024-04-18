use cosmwasm_std::testing::{mock_dependencies, mock_env};
use cosmwasm_std::{coin, Coin, Decimal, Uint128, Uint64};
use white_whale_std::coin;
use white_whale_std::fee::{Fee, PoolFee};
use white_whale_std::pool_network::asset::MINIMUM_LIQUIDITY_AMOUNT;

use crate::tests::robot::TestingRobot;
use crate::tests::test_helpers;

#[test]
fn test_fill_rewards_from_pool_manager() {
    let mut robot = TestingRobot::default();
    let grace_period = Uint64::new(21);
    let creator = robot.sender.clone();
    let epochs = test_helpers::get_epochs();
    let binding = epochs.clone();
    let claimable_epochs = binding
        .iter()
        .rev()
        .take(grace_period.u64() as usize)
        .collect::<Vec<_>>();
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
            vec![coin(1000, "uusdc")],
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
            assert_eq!(res, Uint128::from(18u128));
        },
    );

    robot.create_pair(
        creator.clone(),
        asset_infos.clone(),
        pool_fees.clone(),
        white_whale_std::pool_network::asset::PairType::ConstantProduct,
        Some("whale-uusdc-second".to_string()),
        vec![coin(1000, "uusdc")],
        |result| {
            result.unwrap();
        },
    );

    // Get balance of the bonding manager again it should have the pool creation fee
    robot.query_balance(
        "uwhale".to_string(),
        robot.bonding_manager_addr.clone(),
        |res| {
            assert_eq!(res, Uint128::from(1017u128));
        },
    );

    // create another pair to collect another fee
    robot.create_pair(
        creator.clone(),
        asset_infos,
        pool_fees,
        white_whale_std::pool_network::asset::PairType::ConstantProduct,
        Some("whale-uusdc-third".to_string()),
        vec![coin(1000, "uusdc")],
        |result| {
            result.unwrap();
        },
    );
    // Verify the fee has been collected
    robot.query_balance(
        "uwhale".to_string(),
        robot.bonding_manager_addr.clone(),
        |res| {
            assert_eq!(res, Uint128::from(2016u128));
        },
    );
}
