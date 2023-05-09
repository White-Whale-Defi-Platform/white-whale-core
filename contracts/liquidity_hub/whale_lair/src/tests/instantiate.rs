use cosmwasm_std::{Addr, Decimal, Uint64};

use crate::tests::robot::TestingRobot;
use white_whale::pool_network::asset::AssetInfo;
use white_whale::whale_lair::Config;

#[test]
fn test_instantiate_successfully() {
    let mut robot = TestingRobot::default();

    robot
        .instantiate(
            Uint64::new(1_000u64),
            Decimal::one(),
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
            unbonding_period: Uint64::new(1_000u64),
            growth_rate: Decimal::one(),
            bonding_assets: vec![
                AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                AssetInfo::NativeToken {
                    denom: "bWHALE".to_string(),
                },
            ],
            fee_distributor_addr: Addr::unchecked(""),
        });
}

#[test]
fn test_instantiate_unsuccessfully() {
    let mut robot = TestingRobot::default();

    // over bonding assets limit
    robot.instantiate_err(
        Uint64::new(1_000u64),
        Decimal::one(),
        vec![
            AssetInfo::NativeToken {
                denom: "ampWHALE".to_string(),
            },
            AssetInfo::NativeToken {
                denom: "bWHALE".to_string(),
            },
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
        ],
        &vec![],
        |error| {
            println!("1 --{error:?}");
            println!("2 --{:?}", error.root_cause());
            //println!("3 --{:?}", error.root_cause().downcast_ref::<ContractError>());
            // assert_eq!(
            //     error.root_cause().downcast_ref::<ContractError>().unwrap(),
            //     &ContractError::InvalidBondingAssetsLimit(BONDING_ASSETS_LIMIT, 3));
        },
    );

    // invalid tokens
    robot.instantiate_err(
        Uint64::new(1_000u64),
        Decimal::one(),
        vec![AssetInfo::Token {
            contract_addr: "contract123".to_string(),
        }],
        &vec![],
        |error| {
            println!("1 --{error:?}");
            println!("2 --{:?}", error.root_cause());
            //println!("3 --{:?}", error.root_cause().downcast_ref::<ContractError>());

            // assert_eq!(
            //    error.root_cause().downcast_mut::<ContractError>().unwrap(),
            //    ContractError::InvalidBondingAsset {});
        },
    );
}
