use cosmwasm_std::{Addr, Decimal, Uint64};

use white_whale_std::bonding_manager::Config;

use crate::state::BONDING_ASSETS_LIMIT;
use crate::tests::suite::TestingSuite;
use crate::ContractError;

#[test]
fn test_instantiate_successfully() {
    let mut suite = TestingSuite::default();

    suite
        .instantiate(
            1u64,
            Decimal::one(),
            vec!["ampWHALE".to_string(), "bWHALE".to_string()],
            &vec![],
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

#[test]
fn test_instantiate_unsuccessfully() {
    let mut suite = TestingSuite::default();

    // over bonding assets limit
    suite
        .instantiate_err(
            1u64,
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
            1u64,
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
