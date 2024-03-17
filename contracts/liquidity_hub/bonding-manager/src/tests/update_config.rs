use cosmwasm_std::{Addr, Decimal, Uint128, Uint64};

use white_whale_std::pool_network::asset::AssetInfo;
use white_whale_std::whale_lair::Config;

use crate::tests::robot::TestingRobot;

#[test]
fn test_update_config_successfully() {
    let mut robot = TestingRobot::default();
    let owner = robot.sender.clone();

    robot
        .instantiate_default()
        .assert_config(Config {
            owner: Addr::unchecked("owner"),
            unbonding_period: Uint64::new(1_000_000_000_000u64),
            growth_rate: Decimal::one(),
            bonding_assets: vec![
                AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                AssetInfo::NativeToken {
                    denom: "bWHALE".to_string(),
                },
            ],
            fee_distributor_addr: Addr::unchecked("contract2"),
        })
        .update_config(
            owner.clone(),
            None,
            Some(Uint64::new(500u64)),
            Some(Decimal::from_ratio(
                Uint128::new(1u128),
                Uint128::new(2u128),
            )),
            |_res| {},
        )
        .assert_config(Config {
            owner: owner.clone(),
            unbonding_period: Uint64::new(500u64),
            growth_rate: Decimal::from_ratio(Uint128::new(1u128), Uint128::new(2u128)),
            bonding_assets: vec![
                AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                AssetInfo::NativeToken {
                    denom: "bWHALE".to_string(),
                },
            ],
            fee_distributor_addr: Addr::unchecked("contract2"),
        })
        .update_config(
            owner,
            Some("new_owner".to_string()),
            None,
            Some(Decimal::one()),
            |_res| {},
        )
        .assert_config(Config {
            owner: Addr::unchecked("new_owner"),
            unbonding_period: Uint64::new(500u64),
            growth_rate: Decimal::one(),
            bonding_assets: vec![
                AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                AssetInfo::NativeToken {
                    denom: "bWHALE".to_string(),
                },
            ],
            fee_distributor_addr: Addr::unchecked("contract2"),
        });
}

#[test]
fn test_update_config_unsuccessfully() {
    let mut robot = TestingRobot::default();

    robot
        .instantiate_default()
        .assert_config(Config {
            owner: Addr::unchecked("owner"),
            unbonding_period: Uint64::new(1_000_000_000_000u64),
            growth_rate: Decimal::one(),
            bonding_assets: vec![
                AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                AssetInfo::NativeToken {
                    denom: "bWHALE".to_string(),
                },
            ],
            fee_distributor_addr: Addr::unchecked("contract2"),
        })
        .update_config(
            Addr::unchecked("unauthorized"),
            None,
            Some(Uint64::new(500u64)),
            Some(Decimal::from_ratio(
                Uint128::new(1u128),
                Uint128::new(2u128),
            )),
            |_res| {
                //println!("{:?}", res.unwrap_err().root_cause());
                // assert_eq!(
                //     res.unwrap_err().root_cause().downcast_ref::<ContractError>().unwrap(),
                //     &ContractError::Unauthorized {}
                // );
            },
        )
        .assert_config(Config {
            owner: Addr::unchecked("owner"),
            unbonding_period: Uint64::new(1_000_000_000_000u64),
            growth_rate: Decimal::one(),
            bonding_assets: vec![
                AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                AssetInfo::NativeToken {
                    denom: "bWHALE".to_string(),
                },
            ],
            fee_distributor_addr: Addr::unchecked("contract2"),
        })
        .update_config(
            Addr::unchecked("owner"),
            None,
            Some(Uint64::new(500u64)),
            Some(Decimal::from_ratio(
                Uint128::new(2u128),
                Uint128::new(1u128),
            )),
            |_res| {
                //println!("{:?}", res.unwrap_err().root_cause());
                // assert_eq!(
                //     res.unwrap_err().root_cause().downcast_ref::<ContractError>().unwrap(),
                //     &ContractError::Unauthorized {}
                // );
            },
        )
        .assert_config(Config {
            owner: Addr::unchecked("owner"),
            unbonding_period: Uint64::new(1_000_000_000_000u64),
            growth_rate: Decimal::one(),
            bonding_assets: vec![
                AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                AssetInfo::NativeToken {
                    denom: "bWHALE".to_string(),
                },
            ],
            fee_distributor_addr: Addr::unchecked("contract2"),
        });
}
