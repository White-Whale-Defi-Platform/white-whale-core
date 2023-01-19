use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::Uint128;

use terraswap::asset::{Asset, AssetInfo};

use crate::contract::instantiate;
use crate::msg::InstantiateMsg;
use crate::state::{get_current_epoch, get_expiring_epoch, Epoch, EPOCHS};
use crate::tests::epoch;
use crate::tests::robot::TestingRobot;

// create test for get_current_epoch
#[test]
fn test_get_current_epoch() {
    let robot = TestingRobot::new(mock_dependencies(), mock_env(), mock_info("owner", &[]));

    let grace_period = 2;

    robot
        .instantiate(
            "staking_contract_addr".to_string(),
            "fee_collector_addr".to_string(),
            grace_period,
        )
        .unwrap();

    //robot.add_epochs_to_state(epoch::get_epochs());

    //let current_epoch = get_current_epoch(robot.deps.as_ref()).unwrap();
    //let expiring_epoch = get_expiring_epoch(robot.deps.as_ref()).unwrap();

    //println!("current epoch: {:?}", current_epoch);
    // println!("expiring epoch: {:?}", expiring_epoch.unwrap());
}
