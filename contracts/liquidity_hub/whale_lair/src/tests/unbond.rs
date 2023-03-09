use cosmwasm_std::{coins, Decimal, Timestamp, Uint128};

use white_whale::whale_lair::{
    Asset, AssetInfo, Bond, BondedResponse, BondingWeightResponse, UnbondingResponse,
};

use crate::tests::robot::TestingRobot;

#[test]
fn test_unbond_successfully() {
    let mut robot = TestingRobot::default();
    let sender = robot.sender.clone();

    robot
        .instantiate_default()
        .bond(
            sender.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
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
                weight: Uint128::new(10_000u128),
                global_weight: Uint128::new(10_000u128),
                share: Decimal::one(),
                timestamp: Timestamp::from_nanos(1571797429879305533u64),
            },
        )
        .unbond(
            sender.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
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
                    asset: Asset {
                        info: AssetInfo::NativeToken {
                            denom: "ampWHALE".to_string(),
                        },
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
                bonded_assets: vec![Asset {
                    info: AssetInfo::NativeToken {
                        denom: "ampWHALE".to_string(),
                    },
                    amount: Uint128::new(700u128),
                }],
            },
        )
        .assert_bonding_weight_response(
            sender.to_string(),
            BondingWeightResponse {
                address: sender.to_string(),
                weight: Uint128::new(17_000u128),
                global_weight: Uint128::new(17_000u128),
                share: Decimal::one(),
                timestamp: Timestamp::from_nanos(1571797439879305533u64),
            },
        )
        .fast_forward(10u64)
        .unbond(
            sender.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
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
                        asset: Asset {
                            info: AssetInfo::NativeToken {
                                denom: "ampWHALE".to_string(),
                            },
                            amount: Uint128::new(300u128),
                        },
                        timestamp: Timestamp::from_nanos(1571797429879305533u64),
                        weight: Uint128::zero(),
                    },
                    Bond {
                        asset: Asset {
                            info: AssetInfo::NativeToken {
                                denom: "ampWHALE".to_string(),
                            },
                            amount: Uint128::new(200u128),
                        },
                        timestamp: Timestamp::from_nanos(1571797449879305533u64),
                        weight: Uint128::zero(),
                    },
                ],
            },
        );
}

#[test]
fn test_unbonding_query_pagination() {
    let mut robot = TestingRobot::default();
    let sender = robot.sender.clone();

    robot
        .instantiate_default()
        .bond(
            sender.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            &coins(1_000u128, "ampWHALE"),
            |_res| {},
        )
        .fast_forward(10u64)
        .unbond(
            sender.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                amount: Uint128::new(100u128),
            },
            |_res| {},
        )
        .fast_forward(10u64)
        .unbond(
            sender.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                amount: Uint128::new(100u128),
            },
            |_res| {},
        )
        .fast_forward(10u64)
        .unbond(
            sender.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                amount: Uint128::new(100u128),
            },
            |_res| {},
        )
        .fast_forward(10u64)
        .unbond(
            sender.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
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
                                asset: Asset {
                                    info: AssetInfo::NativeToken {
                                        denom: "ampWHALE".to_string()
                                    },
                                    amount: Uint128::new(100u128),
                                },
                                timestamp: Timestamp::from_nanos(1571797429879305533u64),
                                weight: Uint128::zero(),
                            },
                            Bond {
                                asset: Asset {
                                    info: AssetInfo::NativeToken {
                                        denom: "ampWHALE".to_string()
                                    },
                                    amount: Uint128::new(100u128),
                                },
                                timestamp: Timestamp::from_nanos(1571797439879305533u64),
                                weight: Uint128::zero(),
                            },
                            Bond {
                                asset: Asset {
                                    info: AssetInfo::NativeToken {
                                        denom: "ampWHALE".to_string()
                                    },
                                    amount: Uint128::new(100u128),
                                },
                                timestamp: Timestamp::from_nanos(1571797449879305533u64),
                                weight: Uint128::zero(),
                            },
                            Bond {
                                asset: Asset {
                                    info: AssetInfo::NativeToken {
                                        denom: "ampWHALE".to_string()
                                    },
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
                                asset: Asset {
                                    info: AssetInfo::NativeToken {
                                        denom: "ampWHALE".to_string()
                                    },
                                    amount: Uint128::new(100u128),
                                },
                                timestamp: Timestamp::from_nanos(1571797429879305533u64),
                                weight: Uint128::zero(),
                            },
                            Bond {
                                asset: Asset {
                                    info: AssetInfo::NativeToken {
                                        denom: "ampWHALE".to_string()
                                    },
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
                                asset: Asset {
                                    info: AssetInfo::NativeToken {
                                        denom: "ampWHALE".to_string()
                                    },
                                    amount: Uint128::new(100u128),
                                },
                                timestamp: Timestamp::from_nanos(1571797429879305533u64),
                                weight: Uint128::zero(),
                            },
                            Bond {
                                asset: Asset {
                                    info: AssetInfo::NativeToken {
                                        denom: "ampWHALE".to_string()
                                    },
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
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            &coins(1_000u128, "ampWHALE"),
            |_res| {},
        )
        .fast_forward(10u64)
        .unbond(
            sender.clone(),
            Asset {
                info: AssetInfo::Token {
                    contract_addr: "wrong_token".to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            |res| {
                println!("{:?}", res.unwrap_err().root_cause());
                //assert error is InvalidBondingAsset
            },
        )
        .unbond(
            sender.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "bWHALE".to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            |res| {
                println!("{:?}", res.unwrap_err().root_cause());
                //assert error is NothingToUnbond
            },
        )
        .unbond(
            sender,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                amount: Uint128::new(2_000u128),
            },
            |res| {
                println!("{:?}", res.unwrap_err().root_cause());
                //assert error is InsufficientBond
            },
        );
}
