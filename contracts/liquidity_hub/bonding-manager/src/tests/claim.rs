use std::collections::VecDeque;

use cosmwasm_std::{coin, Uint64};
use white_whale_std::fee::{Fee, PoolFee};
use white_whale_std::pool_network::asset::MINIMUM_LIQUIDITY_AMOUNT;

use crate::tests::robot::TestingRobot;
use crate::tests::test_helpers;
use cosmwasm_std::{coins, Coin, Decimal, Timestamp, Uint128};

use white_whale_std::bonding_manager::{BondedResponse, BondingWeightResponse};

use super::test_helpers::get_epochs;

#[test]
fn test_claimable_epochs() {
    let mut robot = TestingRobot::default();
    let grace_period = Uint64::new(21);

    let epochs = test_helpers::get_epochs();
    let binding = epochs.clone();
    let mut claimable_epochs = binding
        .iter()
        .rev()
        .take(grace_period.u64() as usize)
        .collect::<VecDeque<_>>();
    claimable_epochs.pop_front();

    robot
        .instantiate_default()
        .add_epochs_to_state(epochs)
        .query_claimable_epochs(None, |res| {
            let (_, epochs) = res.unwrap();
            println!("{:?}", epochs);
            assert_eq!(epochs.len(), claimable_epochs.len());
            for (e, a) in epochs.iter().zip(claimable_epochs.iter()) {
                assert_eq!(e, *a);
            }
        });
}

#[test]
fn test_claim_successfully() {
    let mut robot = TestingRobot::default();
    let sender = robot.sender.clone();
    let another_sender = robot.another_sender.clone();
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
    get_epochs();
    robot
        .instantiate_default()
        .bond(
            sender.clone(),
            Coin {
                denom: "ampWHALE".to_string(),
                amount: Uint128::new(1_000u128),
            },
            &coins(1_000u128, "ampWHALE"),
            |_res| {},
        )
        .assert_bonded_response(
            sender.to_string(),
            BondedResponse {
                total_bonded: Uint128::new(1_000u128),
                bonded_assets: vec![Coin {
                    denom: "ampWHALE".to_string(),
                    amount: Uint128::new(1_000u128),
                }],
                first_bonded_epoch_id: Default::default(),
            },
        )
        .fast_forward(10u64)
        .assert_bonding_weight_response(
            sender.to_string(),
            BondingWeightResponse {
                address: sender.to_string(),
                weight: Uint128::new(11_000u128),
                global_weight: Uint128::new(11_000u128),
                share: Decimal::one(),
                timestamp: Timestamp::from_nanos(1571797429879305533u64),
            },
        )
        .fast_forward(10u64)
        .bond(
            sender.clone(),
            Coin {
                denom: "bWHALE".to_string(),
                amount: Uint128::new(3_000u128),
            },
            &coins(3_000u128, "bWHALE"),
            |_res| {},
        )
        .assert_bonded_response(
            sender.to_string(),
            BondedResponse {
                total_bonded: Uint128::new(4_000u128),
                bonded_assets: vec![
                    Coin {
                        denom: "ampWHALE".to_string(),
                        amount: Uint128::new(1_000u128),
                    },
                    Coin {
                        denom: "bWHALE".to_string(),
                        amount: Uint128::new(3_000u128),
                    },
                ],
                first_bonded_epoch_id: Default::default(),
            },
        )
        .fast_forward(10u64)
        .bond(
            another_sender.clone(),
            Coin {
                denom: "ampWHALE".to_string(),
                amount: Uint128::new(5_000u128),
            },
            &coins(5_000u128, "ampWHALE"),
            |_res| {},
        )
        .fast_forward(10u64)
        .assert_bonding_weight_response(
            sender.to_string(),
            BondingWeightResponse {
                address: sender.to_string(),
                weight: Uint128::new(104_000u128),
                global_weight: Uint128::new(269_000u128),
                share: Decimal::from_ratio(104_000u128, 269_000u128),
                timestamp: Timestamp::from_nanos(1571797459879305533u64),
            },
        )
        .assert_bonding_weight_response(
            another_sender.to_string(),
            BondingWeightResponse {
                address: another_sender.to_string(),
                weight: Uint128::new(55_000u128),
                global_weight: Uint128::new(269_000u128),
                share: Decimal::from_ratio(55_000u128, 269_000u128),
                timestamp: Timestamp::from_nanos(1571797459879305533u64),
            },
        )
        .query_total_bonded(|res| {
            let bonded_response = res.unwrap().1;
            assert_eq!(
                bonded_response,
                BondedResponse {
                    total_bonded: Uint128::new(9_000u128),
                    bonded_assets: vec![
                        Coin {
                            denom: "ampWHALE".to_string(),
                            amount: Uint128::new(6_000u128),
                        },
                        Coin {
                            denom: "bWHALE".to_string(),
                            amount: Uint128::new(3_000u128),
                        },
                    ],
                    first_bonded_epoch_id: Default::default(),
                }
            )
        });

    robot.query_claimable_epochs_live(Some(sender.clone()), |res| {
        let (_, epochs) = res.unwrap();
        assert_eq!(epochs.len(), 0);
    });

    robot.create_pair(
        sender.clone(),
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
        sender.clone(),
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
        sender.clone(),
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

    robot
        .create_new_epoch()
        .query_claimable_epochs_live(Some(sender.clone()), |res| {
            let (_, epochs) = res.unwrap();
            assert_eq!(epochs.len(), 1);
        });

    robot.claim(sender, |res| {
        let result = res.unwrap();
        println!("{:?}", result);
        assert!(result.events.iter().any(|event| {
            event
                .attributes
                .iter()
                .any(|attr| attr.key == "amount" && attr.value == "448uwhale")
        }));
    });

    robot.claim(another_sender, |res| {
        let result = res.unwrap();
        println!("{:?}", result);
        assert!(result.events.iter().any(|event| {
            event
                .attributes
                .iter()
                .any(|attr| attr.key == "amount" && attr.value == "560uwhale")
        }));
    });
}
