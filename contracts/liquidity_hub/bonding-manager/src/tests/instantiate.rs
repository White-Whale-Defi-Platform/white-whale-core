use cosmwasm_std::{Addr, Decimal, Uint64};

use crate::state::BONDING_ASSETS_LIMIT;
use crate::tests::robot::TestingRobot;
use crate::ContractError;
use white_whale_std::bonding_manager::Config;

#[test]
fn test_instantiate_successfully() {
    let mut robot = TestingRobot::default();

    robot
        .instantiate(
            Uint64::new(1_000u64),
            Decimal::one(),
            vec!["ampWHALE".to_string(), "bWHALE".to_string()],
            &vec![],
        )
        .assert_config(Config {
            owner: Addr::unchecked("migaloo1h3s5np57a8cxaca3rdjlgu8jzmr2d2zz55s5y3"),
            pool_manager_addr: Addr::unchecked("contract2"),
            distribution_denom: "uwhale".to_string(),
            unbonding_period: Uint64::new(1_000u64),
            growth_rate: Decimal::one(),
            grace_period: Uint64::new(21u64),
            bonding_assets: vec!["ampWHALE".to_string(), "bWHALE".to_string()],
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
    );
}
