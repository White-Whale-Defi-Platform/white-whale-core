use cosmwasm_std::{Addr, Decimal, Uint128, Uint64};

use crate::ContractError;
use white_whale_std::bonding_manager::Config;

use crate::tests::suite::TestingSuite;

#[test]
fn test_update_config_successfully() {
    let mut suite = TestingSuite::default();
    let owner = suite.sender.clone();

    suite
        .instantiate_default()
        .assert_config(Config {
            pool_manager_addr: Addr::unchecked("contract2"),
            epoch_manager_addr: Addr::unchecked("contract0"),
            distribution_denom: "uwhale".to_string(),
            unbonding_period: 1u64,
            growth_rate: Decimal::one(),
            grace_period: 21u64,
            bonding_assets: vec!["ampWHALE".to_string(), "bWHALE".to_string()],
        })
        .assert_owner("migaloo1h3s5np57a8cxaca3rdjlgu8jzmr2d2zz55s5y3".to_string())
        .update_config(
            owner.clone(),
            None,
            None,
            Some(500u64),
            Some(Decimal::from_ratio(
                Uint128::new(1u128),
                Uint128::new(2u128),
            )),
            |_res| {},
        )
        .assert_config(Config {
            pool_manager_addr: Addr::unchecked("contract2"),
            epoch_manager_addr: Addr::unchecked("contract0"),
            distribution_denom: "uwhale".to_string(),
            unbonding_period: 500u64,
            growth_rate: Decimal::from_ratio(Uint128::new(1u128), Uint128::new(2u128)),
            grace_period: 21u64,
            bonding_assets: vec!["ampWHALE".to_string(), "bWHALE".to_string()],
        })
        .update_config(
            owner,
            Some("contract5".to_string()),
            Some("contract6".to_string()),
            None,
            Some(Decimal::one()),
            |_res| {},
        )
        .assert_config(Config {
            pool_manager_addr: Addr::unchecked("contract6"),
            epoch_manager_addr: Addr::unchecked("contract5"),
            distribution_denom: "uwhale".to_string(),
            unbonding_period: 500u64,
            growth_rate: Decimal::one(),
            grace_period: 21u64,
            bonding_assets: vec!["ampWHALE".to_string(), "bWHALE".to_string()],
        });
}

#[test]
fn test_update_config_unsuccessfully() {
    let mut suite = TestingSuite::default();
    let owner = suite.sender.clone();

    suite
        .instantiate_default()
        .assert_config(Config {
            pool_manager_addr: Addr::unchecked("contract2"),
            epoch_manager_addr: Addr::unchecked("contract0"),
            distribution_denom: "uwhale".to_string(),
            unbonding_period: 1u64,
            growth_rate: Decimal::one(),
            grace_period: 21u64,
            bonding_assets: vec!["ampWHALE".to_string(), "bWHALE".to_string()],
        })
        .update_config(
            Addr::unchecked("unauthorized"),
            None,
            None,
            Some(500u64),
            Some(Decimal::from_ratio(
                Uint128::new(1u128),
                Uint128::new(2u128),
            )),
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::OwnershipError { .. } => {}
                    _ => {
                        panic!("Wrong error type, should return ContractError::OwnershipError")
                    }
                }
            },
        )
        .assert_config(Config {
            pool_manager_addr: Addr::unchecked("contract2"),
            epoch_manager_addr: Addr::unchecked("contract0"),
            distribution_denom: "uwhale".to_string(),
            unbonding_period: 1u64,
            growth_rate: Decimal::one(),
            grace_period: 21u64,
            bonding_assets: vec!["ampWHALE".to_string(), "bWHALE".to_string()],
        })
        .update_config(
            owner,
            None,
            None,
            Some(500u64),
            Some(Decimal::from_ratio(
                Uint128::new(2u128),
                Uint128::new(1u128),
            )),
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::InvalidGrowthRate { .. } => {}
                    _ => {
                        panic!("Wrong error type, should return ContractError::InvalidGrowthRate")
                    }
                }
            },
        )
        .assert_config(Config {
            pool_manager_addr: Addr::unchecked("contract2"),
            epoch_manager_addr: Addr::unchecked("contract0"),
            distribution_denom: "uwhale".to_string(),
            unbonding_period: 1u64,
            growth_rate: Decimal::one(),
            grace_period: 21u64,
            bonding_assets: vec!["ampWHALE".to_string(), "bWHALE".to_string()],
        });
}
