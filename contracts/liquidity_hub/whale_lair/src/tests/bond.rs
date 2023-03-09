use std::error::Error;

use cosmwasm_std::testing::{
    mock_dependencies, mock_dependencies_with_balances, mock_env, mock_info,
};
use cosmwasm_std::{coin, coins, Addr, Decimal, StdError, StdResult, Timestamp, Uint128};

use white_whale::whale_lair::{Asset, AssetInfo, BondedResponse, BondingWeightResponse, Config};

use crate::tests::robot::TestingRobot;
use crate::ContractError;

#[test]
fn test_bond_successfully() {
    let mut robot = TestingRobot::default();
    let sender = robot.sender.clone();
    let another_sender = robot.another_sender.clone();

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
            |res| {},
        )
        .assert_bonded_response(
            sender.to_string(),
            BondedResponse {
                total_bonded: Uint128::new(1_000u128),
                bonded_assets: vec![Asset {
                    info: AssetInfo::NativeToken {
                        denom: "ampWHALE".to_string(),
                    },
                    amount: Uint128::new(1_000u128),
                }],
            },
        )
        .fast_forward(10u64)
        .assert_bonding_weight_response(
            sender.clone().to_string(),
            BondingWeightResponse {
                address: sender.clone().to_string(),
                weight: Uint128::new(10_000u128),
                global_weight: Uint128::new(10_000u128),
                share: Decimal::one(),
                timestamp: Timestamp::from_nanos(1571797429879305533u64),
            },
        )
        .fast_forward(10u64)
        .bond(
            sender.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "bWHALE".to_string(),
                },
                amount: Uint128::new(3_000u128),
            },
            &coins(3_000u128, "bWHALE"),
            |res| {},
        )
        .assert_bonded_response(
            sender.to_string(),
            BondedResponse {
                total_bonded: Uint128::new(4_000u128),
                bonded_assets: vec![
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: "ampWHALE".to_string(),
                        },
                        amount: Uint128::new(1_000u128),
                    },
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: "bWHALE".to_string(),
                        },
                        amount: Uint128::new(3_000u128),
                    },
                ],
            },
        )
        .fast_forward(10u64)
        .assert_bonding_weight_response(
            sender.clone().to_string(),
            BondingWeightResponse {
                address: sender.clone().to_string(),
                weight: Uint128::new(60_000u128),
                global_weight: Uint128::new(60_000u128),
                share: Decimal::one(),
                timestamp: Timestamp::from_nanos(1571797449879305533u64),
            },
        )
        .bond(
            another_sender.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                amount: Uint128::new(5_000u128),
            },
            &coins(5_000u128, "ampWHALE"),
            |res| {},
        )
        .fast_forward(10u64)
        .assert_bonding_weight_response(
            sender.clone().to_string(),
            BondingWeightResponse {
                address: sender.to_string(),
                weight: Uint128::new(100_000u128),
                global_weight: Uint128::new(150_000u128),
                share: Decimal::from_ratio(100_000u128, 150_000u128),
                timestamp: Timestamp::from_nanos(1571797459879305533u64),
            },
        )
        .assert_bonding_weight_response(
            another_sender.clone().to_string(),
            BondingWeightResponse {
                address: another_sender.to_string(),
                weight: Uint128::new(50_000u128),
                global_weight: Uint128::new(150_000u128),
                share: Decimal::from_ratio(50_000u128, 150_000u128),
                timestamp: Timestamp::from_nanos(1571797459879305533u64),
            },
        );
}

#[test]
fn test_bond_wrong_asset() {
    let mut robot = TestingRobot::default();
    let sender = robot.sender.clone();

    robot
        .instantiate_default()
        .bond(
            sender.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "bWHALE".to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            &vec![coin(5_000u128, "bWHALE")],
            |res| {
                println!("{:?}", res.unwrap_err().root_cause());
                //assert error is AssetMismatch
            },
        )
        .bond(
            sender.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "non_whitelisted_asset".to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            &vec![
                coin(1_000u128, "non_whitelisted_asset"),
                coin(1_000u128, "ampWHALE"),
            ],
            |res| {
                println!("{:?}", res.unwrap_err().root_cause());
                //assert error is AssetMismatch
            },
        )
        .bond(
            sender.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            &vec![coin(1_000u128, "non_whitelisted_asset")],
            |res| {
                println!("{:?}", res.unwrap_err().root_cause());
                //assert error is AssetMismatch
            },
        )
        .bond(
            sender.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "non_whitelisted_asset".to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            &vec![coin(1_000u128, "non_whitelisted_asset")],
            |res| {
                println!("{:?}", res.unwrap_err().root_cause());
                //assert error is AssetMismatch
            },
        )
        .bond(
            sender.clone(),
            Asset {
                info: AssetInfo::Token {
                    contract_addr: "contract123".to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            &vec![],
            |res| {
                println!("{:?}", res.unwrap_err().root_cause());
                //assert error is InvalidBondingAsset
            },
        )
        .bond(
            sender.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "bWHALE".to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            &vec![],
            |res| {
                println!("{:?}", res.unwrap_err().root_cause());
                //assert error is AssetMismatch
            },
        );
}
