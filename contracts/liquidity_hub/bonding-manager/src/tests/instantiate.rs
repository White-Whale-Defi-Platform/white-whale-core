use cosmwasm_std::{Addr, Decimal, Uint64};

use white_whale_std::bonding_manager::Config;

use crate::state::BONDING_ASSETS_LIMIT;
use crate::tests::suite::TestingSuite;
use crate::ContractError;

#[test]
fn test_instantiate_successfully() {
    let mut robot = TestingSuite::default();

    robot
        .instantiate(
            Uint64::new(1_000u64),
            Decimal::one(),
            vec!["ampWHALE".to_string(), "bWHALE".to_string()],
            &vec![],
        )
        .assert_config(Config {
            pool_manager_addr: Addr::unchecked("contract2"),
            epoch_manager_addr: Addr::unchecked("contract0"),
            distribution_denom: "uwhale".to_string(),
            unbonding_period: Uint64::new(1_000u64),
            growth_rate: Decimal::one(),
            grace_period: Uint64::new(21u64),
            bonding_assets: vec!["ampWHALE".to_string(), "bWHALE".to_string()],
        });
}

#[test]
fn test_instantiate_unsuccessfully() {
    let mut robot = TestingSuite::default();

    // over bonding assets limit
    robot
        .instantiate_err(
            Uint64::new(1_000u64),
            Decimal::one(),
            vec![
                "ampWHALE".to_string(),
                "bWHALE".to_string(),
                "uwhale".to_string(),
            ],
            &vec![],
            |error| {
                assert_eq!(
                    error.root_cause().downcast_ref::<ContractError>().unwrap(),
                    &ContractError::InvalidBondingAssetsLimit(BONDING_ASSETS_LIMIT, 3)
                );
            },
        )
        .instantiate_err(
            Uint64::new(1_000u64),
            Decimal::percent(200),
            vec!["ampWHALE".to_string(), "bWHALE".to_string()],
            &vec![],
            |error| {
                assert_eq!(
                    error.root_cause().downcast_ref::<ContractError>().unwrap(),
                    &ContractError::InvalidGrowthRate
                );
            },
        );
}
