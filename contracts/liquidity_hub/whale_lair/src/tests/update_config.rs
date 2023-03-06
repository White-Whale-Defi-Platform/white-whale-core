use cosmwasm_std::testing::{
    mock_dependencies, mock_dependencies_with_balances, mock_env, mock_info,
};
use cosmwasm_std::{coin, coins, Addr, Uint128};

use white_whale::whale_lair::{AssetInfo, Config};

use crate::tests::robot::TestingRobot;
use crate::ContractError;

#[test]
fn test_update_config_successfully() {
    let mut robot = TestingRobot::default();
    let owner = robot.sender.clone();

    robot
        .instantiate(
            1_000u64,
            1u8,
            vec![
                AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                AssetInfo::NativeToken {
                    denom: "bWHALE".to_string(),
                },
            ],
            &vec![],
        )
        .assert_config(Config {
            owner: Addr::unchecked("owner"),
            unbonding_period: 1_000u64,
            growth_rate: 1u8,
            bonding_assets: vec![
                AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                AssetInfo::NativeToken {
                    denom: "bWHALE".to_string(),
                },
            ],
        })
        .update_config(owner.clone(), None, Some(500u64), Some(2u8), |res| {})
        .assert_config(Config {
            owner: owner.clone(),
            unbonding_period: 500u64,
            growth_rate: 2u8,
            bonding_assets: vec![
                AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                AssetInfo::NativeToken {
                    denom: "bWHALE".to_string(),
                },
            ],
        })
        .update_config(
            owner,
            Some("new_owner".to_string()),
            None,
            Some(1u8),
            |res| {},
        )
        .assert_config(Config {
            owner: Addr::unchecked("new_owner"),
            unbonding_period: 500u64,
            growth_rate: 1u8,
            bonding_assets: vec![
                AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                AssetInfo::NativeToken {
                    denom: "bWHALE".to_string(),
                },
            ],
        });
}

#[test]
fn test_update_config_unsuccessfully() {
    let mut robot = TestingRobot::default();

    robot
        .instantiate(
            1_000u64,
            1u8,
            vec![
                AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                AssetInfo::NativeToken {
                    denom: "bWHALE".to_string(),
                },
            ],
            &vec![],
        )
        .assert_config(Config {
            owner: Addr::unchecked("owner"),
            unbonding_period: 1_000u64,
            growth_rate: 1u8,
            bonding_assets: vec![
                AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                AssetInfo::NativeToken {
                    denom: "bWHALE".to_string(),
                },
            ],
        })
        .update_config(
            Addr::unchecked("unauthorized"),
            None,
            Some(500u64),
            Some(2u8),
            |res| {
                //println!("{:?}", res.unwrap_err().root_cause());
                // assert_eq!(
                //     res.unwrap_err().root_cause().downcast_ref::<ContractError>().unwrap(),
                //     &ContractError::Unauthorized {}
                // );
            },
        )
        .assert_config(Config {
            owner: Addr::unchecked("owner"),
            unbonding_period: 1_000u64,
            growth_rate: 1u8,
            bonding_assets: vec![
                AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                AssetInfo::NativeToken {
                    denom: "bWHALE".to_string(),
                },
            ],
        });
}
