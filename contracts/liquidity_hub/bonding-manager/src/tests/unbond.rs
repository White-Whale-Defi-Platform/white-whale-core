use cosmwasm_std::{coins, Coin, Decimal, Timestamp, Uint128, Uint64};

use white_whale_std::bonding_manager::{
    Bond, BondedResponse, BondingWeightResponse, UnbondingResponse,
};

use crate::tests::robot::TestingRobot;

#[test]
#[track_caller]
fn test_unbond_successfully() {
    let mut robot = TestingRobot::default();
    let sender = robot.sender.clone();
    let another_sender = robot.another_sender.clone();

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
        .unbond(
            sender.clone(),
            Coin {
                denom: "ampWHALE".to_string(),
                amount: Uint128::new(300u128),
            },
            |_res| {},
        )
        .fast_forward(10u64)
        .assert_unbonding_response(
            sender.to_string(),
            "ampWHALE".to_string(),
            UnbondingResponse {
                total_amount: Uint128::new(300u128),
                unbonding_requests: vec![Bond {
                    asset: Coin {
                        denom: "ampWHALE".to_string(),
                        amount: Uint128::new(300u128),
                    },
                    timestamp: Timestamp::from_nanos(1571797429879305533u64),
                    weight: Uint128::zero(),
                }],
            },
        )
        .assert_unbonding_response(
            sender.to_string(),
            "bWHALE".to_string(),
            UnbondingResponse {
                total_amount: Uint128::zero(),
                unbonding_requests: vec![],
            },
        )
        .assert_bonded_response(
            sender.to_string(),
            BondedResponse {
                total_bonded: Uint128::new(700u128),
                bonded_assets: vec![Coin {
                    denom: "ampWHALE".to_string(),
                    amount: Uint128::new(700u128),
                }],
                first_bonded_epoch_id: Uint64::one(),
            },
        )
        .assert_bonding_weight_response(
            sender.to_string(),
            BondingWeightResponse {
                address: sender.to_string(),
                weight: Uint128::new(14_700u128),
                global_weight: Uint128::new(14_700u128),
                share: Decimal::one(),
                timestamp: Timestamp::from_nanos(1571797439879305533u64),
            },
        )
        .fast_forward(10u64)
        .unbond(
            sender.clone(),
            Coin {
                denom: "ampWHALE".to_string(),
                amount: Uint128::new(200u128),
            },
            |_res| {},
        )
        .assert_unbonding_response(
            sender.to_string(),
            "ampWHALE".to_string(),
            UnbondingResponse {
                total_amount: Uint128::new(500u128),
                unbonding_requests: vec![
                    Bond {
                        asset: Coin {
                            denom: "ampWHALE".to_string(),
                            amount: Uint128::new(300u128),
                        },
                        timestamp: Timestamp::from_nanos(1571797429879305533u64),
                        weight: Uint128::zero(),
                    },
                    Bond {
                        asset: Coin {
                            denom: "ampWHALE".to_string(),
                            amount: Uint128::new(200u128),
                        },
                        timestamp: Timestamp::from_nanos(1571797449879305533u64),
                        weight: Uint128::zero(),
                    },
                ],
            },
        )
        .bond(
            another_sender.clone(),
            Coin {
                denom: "bWHALE".to_string(),
                amount: Uint128::new(1_000u128),
            },
            &coins(1_000u128, "bWHALE"),
            |_res| {},
        )
        .query_total_bonded(|res| {
            let bonded_response = res.unwrap().1;
            assert_eq!(
                bonded_response,
                BondedResponse {
                    total_bonded: Uint128::new(1_500u128),
                    bonded_assets: vec![
                        Coin {
                            denom: "ampWHALE".to_string(),
                            amount: Uint128::new(500u128),
                        },
                        Coin {
                            denom: "bWHALE".to_string(),
                            amount: Uint128::new(1_000u128),
                        },
                    ],
                    first_bonded_epoch_id: Default::default(),
                }
            )
        });
}

#[test]
fn test_unbond_all_successfully() {
    let mut robot = TestingRobot::default();
    let sender = robot.sender.clone();

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
        .unbond(
            sender.clone(),
            Coin {
                denom: "ampWHALE".to_string(),
                amount: Uint128::new(1000u128),
            },
            |res| {
                res.unwrap();
            },
        );
}

#[test]
#[track_caller]
fn test_unbonding_query_pagination() {
    let mut robot = TestingRobot::default();
    let sender = robot.sender.clone();

    robot
        .instantiate_default()
        .bond(
            sender.clone(),
            Coin {
                // Change 'Asset' to 'Coin'
                denom: "ampWHALE".to_string(),
                amount: Uint128::new(1_000u128),
            },
            &coins(1_000u128, "ampWHALE"),
            |_res| {},
        )
        .fast_forward(10u64)
        .unbond(
            sender.clone(),
            Coin {
                // Change 'Asset' to 'Coin'
                denom: "ampWHALE".to_string(),
                amount: Uint128::new(100u128),
            },
            |_res| {},
        )
        .fast_forward(10u64)
        .unbond(
            sender.clone(),
            Coin {
                // Change 'Asset' to 'Coin'
                denom: "ampWHALE".to_string(),
                amount: Uint128::new(100u128),
            },
            |_res| {},
        )
        .fast_forward(10u64)
        .unbond(
            sender.clone(),
            Coin {
                // Change 'Asset' to 'Coin'
                denom: "ampWHALE".to_string(),
                amount: Uint128::new(100u128),
            },
            |_res| {},
        )
        .fast_forward(10u64)
        .unbond(
            sender.clone(),
            Coin {
                // Change 'Asset' to 'Coin'
                denom: "ampWHALE".to_string(),
                amount: Uint128::new(100u128),
            },
            |_res| {},
        )
        .fast_forward(10u64)
        .query_unbonding(
            sender.to_string(),
            "ampWHALE".to_string(),
            None,
            None,
            |res| {
                assert_eq!(
                    res.unwrap().1,
                    UnbondingResponse {
                        total_amount: Uint128::new(400u128),
                        unbonding_requests: vec![
                            Bond {
                                asset: Coin {
                                    // Change 'Asset' to 'Coin'
                                    denom: "ampWHALE".to_string(),
                                    amount: Uint128::new(100u128),
                                },
                                timestamp: Timestamp::from_nanos(1571797429879305533u64),
                                weight: Uint128::zero(),
                            },
                            Bond {
                                asset: Coin {
                                    // Change 'Asset' to 'Coin'
                                    denom: "ampWHALE".to_string(),
                                    amount: Uint128::new(100u128),
                                },
                                timestamp: Timestamp::from_nanos(1571797439879305533u64),
                                weight: Uint128::zero(),
                            },
                            Bond {
                                asset: Coin {
                                    // Change 'Asset' to 'Coin'
                                    denom: "ampWHALE".to_string(),
                                    amount: Uint128::new(100u128),
                                },
                                timestamp: Timestamp::from_nanos(1571797449879305533u64),
                                weight: Uint128::zero(),
                            },
                            Bond {
                                asset: Coin {
                                    // Change 'Asset' to 'Coin'
                                    denom: "ampWHALE".to_string(),
                                    amount: Uint128::new(100u128),
                                },
                                timestamp: Timestamp::from_nanos(1571797459879305533u64),
                                weight: Uint128::zero(),
                            },
                        ],
                    }
                )
            },
        )
        .query_unbonding(
            sender.to_string(),
            "ampWHALE".to_string(),
            None,
            Some(2u8),
            |res| {
                assert_eq!(
                    res.unwrap().1,
                    UnbondingResponse {
                        total_amount: Uint128::new(200u128),
                        unbonding_requests: vec![
                            Bond {
                                asset: Coin {
                                    // Change 'Asset' to 'Coin'
                                    denom: "ampWHALE".to_string(),
                                    amount: Uint128::new(100u128),
                                },
                                timestamp: Timestamp::from_nanos(1571797429879305533u64),
                                weight: Uint128::zero(),
                            },
                            Bond {
                                asset: Coin {
                                    // Change 'Asset' to 'Coin'
                                    denom: "ampWHALE".to_string(),
                                    amount: Uint128::new(100u128),
                                },
                                timestamp: Timestamp::from_nanos(1571797439879305533u64),
                                weight: Uint128::zero(),
                            },
                        ],
                    }
                )
            },
        )
        .query_unbonding(
            sender.to_string(),
            "ampWHALE".to_string(),
            Some(12365u64), // start after the block height of the last item in the previous query
            Some(2u8),
            |res| {
                assert_eq!(
                    res.unwrap().1,
                    UnbondingResponse {
                        total_amount: Uint128::new(200u128),
                        unbonding_requests: vec![
                            Bond {
                                asset: Coin {
                                    denom: "ampWHALE".to_string(),
                                    amount: Uint128::new(100u128),
                                },
                                timestamp: Timestamp::from_nanos(1571797429879305533u64),
                                weight: Uint128::zero(),
                            },
                            Bond {
                                asset: Coin {
                                    denom: "ampWHALE".to_string(),
                                    amount: Uint128::new(100u128),
                                },
                                timestamp: Timestamp::from_nanos(1571797439879305533u64),
                                weight: Uint128::zero(),
                            },
                        ],
                    }
                )
            },
        );
}

#[test]
fn test_unbond_unsuccessfully() {
    let mut robot = TestingRobot::default();
    let sender = robot.sender.clone();

    robot
        .instantiate_default()
        .bond(
            sender.clone(),
            Coin {
                // Change 'Asset' to 'Coin'
                denom: "ampWHALE".to_string(),
                amount: Uint128::new(1_000u128),
            },
            &coins(1_000u128, "ampWHALE"),
            |_res| {},
        )
        .fast_forward(10u64)
        .unbond(
            sender.clone(),
            Coin {
                // Change 'Asset' to 'Coin'
                denom: "wrong_token".to_string(),
                amount: Uint128::new(1_000u128),
            },
            |res| {
                println!("{:?}", res.unwrap_err().root_cause());
                //assert error is InvalidBondingAsset
            },
        )
        .unbond(
            sender.clone(),
            Coin {
                // Change 'Asset' to 'Coin'
                denom: "bWHALE".to_string(),
                amount: Uint128::new(1_000u128),
            },
            |res| {
                println!("{:?}", res.unwrap_err().root_cause());
                //assert error is NothingToUnbond
            },
        )
        .unbond(
            sender.clone(),
            Coin {
                // Change 'Asset' to 'Coin'
                denom: "ampWHALE".to_string(),
                amount: Uint128::new(2_000u128),
            },
            |res| {
                println!("{:?}", res.unwrap_err().root_cause());
                //assert error is InsufficientBond
            },
        )
        .unbond(
            sender,
            Coin {
                // Change 'Asset' to 'Coin'
                denom: "ampWHALE".to_string(),
                amount: Uint128::new(0u128),
            },
            |res| {
                println!("{:?}", res.unwrap_err().root_cause());
                //assert error is InvalidUnbondingAmount
            },
        );
}
