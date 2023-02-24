use cosmwasm_std::testing::{
    mock_dependencies, mock_dependencies_with_balances, mock_env, mock_info,
};
use cosmwasm_std::{coin, coins, Addr, Uint128};

use white_whale::whale_lair::Config;

use crate::tests::robot::TestingRobot;
use crate::ContractError;

#[test]
fn test_update_config() {
    let mut robot = TestingRobot::default();

    robot
        .instantiate(
            1_000u64,
            1u8,
            "uwhale".to_string(),
            &vec![coin(1u128, "uwhale")],
        )
        .update_config();
}
