use cosmwasm_std::testing::{mock_dependencies, mock_env};
use cosmwasm_std::Uint64;
use crate::tests::epoch;
use crate::tests::robot::TestingRobot;

#[test]
fn test_claimable_epochs() {
    let mut robot = TestingRobot::new(mock_dependencies(), mock_env());
    let grace_period = Uint64::new(2);

    let epochs = epoch::get_epochs();
    let binding = epochs.clone();
    let claimable_epochs = binding
        .iter()
        .rev()
        .take(grace_period.u64() as usize)
        .collect::<Vec<_>>();

    robot
        .instantiate_default()
        .add_epochs_to_state(epochs)
        .query_claimable_epochs(|res| {
            let (_, epochs) = res.unwrap();

            assert_eq!(epochs.len(), claimable_epochs.len());
            for (e, a) in epochs.iter().zip(claimable_epochs.iter()) {
                assert_eq!(e, *a);
            }
        });
}
