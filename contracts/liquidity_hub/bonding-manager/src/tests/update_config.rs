use cosmwasm_std::{Addr, Decimal, Uint128, Uint64};

use white_whale_std::bonding_manager::Config;

use crate::tests::robot::TestingRobot;

#[test]
fn test_update_config_successfully() {
    let mut robot = TestingRobot::default();
    let owner = robot.sender.clone();

    robot
        .instantiate_default()
        .assert_config(Config {
            owner: Addr::unchecked("migaloo1h3s5np57a8cxaca3rdjlgu8jzmr2d2zz55s5y3"),
            pool_manager_addr: Addr::unchecked("contract2"),
            distribution_denom: "uwhale".to_string(),
            unbonding_period: Uint64::new(1_000_000_000_000u64),
            growth_rate: Decimal::one(),
            grace_period: Uint64::new(21u64),
            bonding_assets: vec!["ampWHALE".to_string(), "bWHALE".to_string()],
        })
        .update_config(
            owner.clone(),
            None,
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
            pool_manager_addr: Addr::unchecked("contract2"),
            distribution_denom: "uwhale".to_string(),
            unbonding_period: Uint64::new(500u64),
            growth_rate: Decimal::from_ratio(Uint128::new(1u128), Uint128::new(2u128)),
            grace_period: Uint64::new(21u64),
            bonding_assets: vec!["ampWHALE".to_string(), "bWHALE".to_string()],
        })
        .update_config(
            owner,
            Some("new_owner".to_string()),
            None,
            None,
            Some(Decimal::one()),
            |_res| {},
        )
        .assert_config(Config {
            owner: Addr::unchecked("new_owner"),
            pool_manager_addr: Addr::unchecked("contract2"),
            distribution_denom: "uwhale".to_string(),
            unbonding_period: Uint64::new(500u64),
            growth_rate: Decimal::one(),
            grace_period: Uint64::new(21u64),
            bonding_assets: vec!["ampWHALE".to_string(), "bWHALE".to_string()],
        });
}

#[test]
fn test_update_config_unsuccessfully() {
    let mut robot = TestingRobot::default();

    robot
        .instantiate_default()
        .assert_config(Config {
            owner: Addr::unchecked("migaloo1h3s5np57a8cxaca3rdjlgu8jzmr2d2zz55s5y3"),
            pool_manager_addr: Addr::unchecked("contract2"),
            distribution_denom: "uwhale".to_string(),
            unbonding_period: Uint64::new(1_000_000_000_000u64),
            growth_rate: Decimal::one(),
            grace_period: Uint64::new(21u64),
            bonding_assets: vec!["ampWHALE".to_string(), "bWHALE".to_string()],
        })
        .update_config(
            Addr::unchecked("unauthorized"),
            None,
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
            owner: Addr::unchecked("migaloo1h3s5np57a8cxaca3rdjlgu8jzmr2d2zz55s5y3"),
            pool_manager_addr: Addr::unchecked("contract2"),
            distribution_denom: "uwhale".to_string(),
            unbonding_period: Uint64::new(1_000_000_000_000u64),
            growth_rate: Decimal::one(),
            grace_period: Uint64::new(21u64),
            bonding_assets: vec!["ampWHALE".to_string(), "bWHALE".to_string()],
        })
        .update_config(
            Addr::unchecked("owner"),
            None,
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
            owner: Addr::unchecked("migaloo1h3s5np57a8cxaca3rdjlgu8jzmr2d2zz55s5y3"),
            pool_manager_addr: Addr::unchecked("contract2"),
            distribution_denom: "uwhale".to_string(),
            unbonding_period: Uint64::new(1_000_000_000_000u64),
            growth_rate: Decimal::one(),
            grace_period: Uint64::new(21u64),
            bonding_assets: vec!["ampWHALE".to_string(), "bWHALE".to_string()],
        });
}
