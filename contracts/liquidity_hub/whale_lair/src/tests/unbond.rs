use std::error::Error;

use cosmwasm_std::testing::{
    mock_dependencies, mock_dependencies_with_balances, mock_env, mock_info,
};
use cosmwasm_std::{coin, coins, Addr, Decimal, StdError, StdResult, Uint128};

use white_whale::whale_lair::{
    Asset, AssetInfo, Bond, BondedResponse, BondingWeightResponse, Config, UnbondingResponse,
};

use crate::tests::robot::TestingRobot;
use crate::ContractError;

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
            |res| {},
        )
        .fast_forward(10u64)
        .assert_bonding_weight_response(
            sender.clone().to_string(),
            BondingWeightResponse {
                address: sender.clone().to_string(),
                weight: Uint128::new(10_000u128),
                global_weight: Uint128::new(10_000u128),
                share: Decimal::one(),
                block_height: 12355u64,
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
            |res| {},
        )
        .fast_forward(10u64)
        .assert_unbonding_response(
            sender.clone().to_string(),
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
                    block_height: 12355u64,
                    weight: Uint128::zero(),
                }],
            },
        )
        .assert_unbonding_response(
            sender.clone().to_string(),
            "bWHALE".to_string(),
            UnbondingResponse {
                total_amount: Uint128::zero(),
                unbonding_requests: vec![],
            },
        )
        .assert_bonded_response(
            sender.clone().to_string(),
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
            sender.clone().to_string(),
            BondingWeightResponse {
                address: sender.clone().to_string(),
                weight: Uint128::new(17_000u128),
                global_weight: Uint128::new(17_000u128),
                share: Decimal::one(),
                block_height: 12365u64,
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
            |res| {},
        )
        .assert_unbonding_response(
            sender.clone().to_string(),
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
                        block_height: 12355u64,
                        weight: Uint128::zero(),
                    },
                    Bond {
                        asset: Asset {
                            info: AssetInfo::NativeToken {
                                denom: "ampWHALE".to_string(),
                            },
                            amount: Uint128::new(200u128),
                        },
                        block_height: 12375u64,
                        weight: Uint128::zero(),
                    },
                ],
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
            |res| {},
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
            sender.clone(),
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
