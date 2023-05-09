use cosmwasm_std::testing::{mock_dependencies, mock_env};
use cosmwasm_std::{Addr, Uint64};

use crate::tests::robot::TestingRobot;
use crate::tests::test_helpers;

#[test]
fn test_claimable_epochs() {
    let mut robot = TestingRobot::new(mock_dependencies(), mock_env());
    let grace_period = Uint64::new(2);

    let epochs = test_helpers::get_epochs();
    let binding = epochs.clone();
    let claimable_epochs = binding
        .iter()
        .rev()
        .take(grace_period.u64() as usize)
        .collect::<Vec<_>>();

    robot
        .instantiate_default()
        .add_epochs_to_state(epochs)
        .query_claimable_epochs(None, |res| {
            let (_, epochs) = res.unwrap();

            assert_eq!(epochs.len(), claimable_epochs.len());
            for (e, a) in epochs.iter().zip(claimable_epochs.iter()) {
                assert_eq!(e, *a);
            }
        });
}

#[test]
fn test_claimable_epochs_for_user() {
    let mut robot = TestingRobot::new(mock_dependencies(), mock_env());
    let epochs = test_helpers::get_epochs();

    robot
        .instantiate_default() //grace period = 2
        .add_epochs_to_state(epochs)
        .query_claimable_epochs(Some(Addr::unchecked("owner")), |res| {
            let (_, epochs) = res.unwrap();
            // the user has not bonded yet
            assert_eq!(epochs.len(), 0usize);
        })
        // simulate that he bonded at epoch 2
        .add_last_claimed_epoch_to_state(Addr::unchecked("owner"), Uint64::new(2))
        .query_claimable_epochs(Some(Addr::unchecked("owner")), |res| {
            let (_, epochs) = res.unwrap();
            assert_eq!(epochs.len(), 1usize);
        })
        .add_last_claimed_epoch_to_state(Addr::unchecked("owner"), Uint64::new(3))
        .query_claimable_epochs(Some(Addr::unchecked("owner")), |res| {
            let (_, epochs) = res.unwrap();
            assert!(epochs.is_empty());
        });
}
