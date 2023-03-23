use cosmwasm_std::testing::{mock_dependencies, mock_env};
use cosmwasm_std::Uint64;

use white_whale::fee_distributor::Epoch;

use crate::tests::robot::TestingRobot;
use crate::tests::test_helpers;

#[test]
fn test_current_epoch_no_epochs() {
    let mut robot = TestingRobot::new(mock_dependencies(), mock_env());

    robot
        .instantiate_default()
        .assert_current_epoch(&Epoch::default())
        .query_epoch(Uint64::new(10), |res| {
            // epoch 10 doesn't exist, it should return the default value
            let (_, epoch) = res.unwrap();
            assert_eq!(epoch, Epoch::default());
        });
}

#[test]
fn test_expiring_epoch() {
    let mut robot = TestingRobot::new(mock_dependencies(), mock_env());
    let epochs = test_helpers::get_epochs();

    robot
        .instantiate_default()
        .add_epochs_to_state(epochs.clone())
        .assert_expiring_epoch(Some(&epochs[1]));
}
