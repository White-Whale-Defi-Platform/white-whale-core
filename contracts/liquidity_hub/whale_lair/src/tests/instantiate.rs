use cosmwasm_std::testing::{
    mock_dependencies, mock_dependencies_with_balances, mock_env, mock_info,
};
use cosmwasm_std::{coin, Addr, Uint128};

use white_whale::whale_lair::Config;

use crate::tests::robot::TestingRobot;

#[test]
fn test_instantiate() {
    let mut robot = TestingRobot::default();

    robot.instantiate_err(1_000u64, 1u8, "uwhale".to_string(), &vec![]);

    robot
        .instantiate(
            1_000u64,
            1u8,
            "uwhale".to_string(),
            &vec![coin(1u128, "uwhale")],
        )
        .assert_config(Config {
            owner: Addr::unchecked("owner"),
            unbonding_period: 1_000u64,
            growth_rate: 1u8,
            bonding_assets: "uwhale".to_string(),
        });
}
