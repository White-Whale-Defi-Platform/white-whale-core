use crate::incentive::commands::{compute_start_from_epoch_for_incentive, compute_user_weights};
use crate::state::ADDRESS_LP_WEIGHT_HISTORY;
use cosmwasm_std::{Addr, Coin, Storage, Uint128};
use white_whale_std::incentive_manager::{Curve, EpochId, Incentive};
use white_whale_std::pool_network::asset::{Asset, AssetInfo};
use white_whale_std::pool_network::mock_querier::mock_dependencies;

#[test]
fn compute_start_from_epoch_for_incentive_successfully() {
    let mut deps = mock_dependencies(&[]);
    let user = Addr::unchecked("user");

    let mut incentive = Incentive {
        identifier: "incentive".to_string(),
        owner: user.clone(),
        lp_denom: "lp".to_string(),
        incentive_asset: Coin {
            denom: "incentive".to_string(),
            amount: Uint128::new(1_000),
        },
        claimed_amount: Default::default(),
        emission_rate: Default::default(),
        curve: Curve::Linear,
        start_epoch: 10,
        preliminary_end_epoch: 20,
        last_epoch_claimed: 9,
    };

    let current_epoch_id = 12u64;

    // Mimics the scenario where the user has never claimed before, but opened a position before the incentive
    // went live
    let first_user_weight_epoch_id = 8;
    ADDRESS_LP_WEIGHT_HISTORY
        .save(
            &mut deps.storage,
            (&user, first_user_weight_epoch_id),
            &Uint128::one(),
        )
        .unwrap();

    let start_from_epoch =
        compute_start_from_epoch_for_incentive(&deps.storage, &incentive, None, &user).unwrap();

    // the function should return the start epoch of the incentive
    assert_eq!(start_from_epoch, 10);

    // Mimics the scenario where the user has never claimed before, but opened a position after the incentive
    // went live
    incentive.start_epoch = 5u64;
    let start_from_epoch =
        compute_start_from_epoch_for_incentive(&deps.storage, &incentive, None, &user).unwrap();

    // the function should return the first epoch the user has a weight
    assert_eq!(start_from_epoch, 8);

    // Mimics the scenario where the user has claimed already, after the incentive went live, i.e. the user
    // has already partially claimed this incentive
    incentive.start_epoch = 10u64;
    let start_from_epoch =
        compute_start_from_epoch_for_incentive(&deps.storage, &incentive, Some(12u64), &user)
            .unwrap();

    // the function should return the next epoch after the last claimed one
    assert_eq!(start_from_epoch, 13);

    // Mimics the scenario where the user has claimed already, before the incentive went live, i.e. the user
    // has not claimed this incentive at all
    incentive.start_epoch = 15u64;
    let start_from_epoch =
        compute_start_from_epoch_for_incentive(&deps.storage, &incentive, Some(12u64), &user)
            .unwrap();

    // the function should return the start epoch of the incentive
    assert_eq!(start_from_epoch, 15);

    // Mimics the scenario where the user has claimed the epoch the incentives went live
    incentive.start_epoch = 15u64;
    let start_from_epoch =
        compute_start_from_epoch_for_incentive(&deps.storage, &incentive, Some(15u64), &user)
            .unwrap();

    // the function should return the next epoch after the last claimed one
    assert_eq!(start_from_epoch, 16);
}

#[test]
fn compute_user_weights_successfully() {
    let mut deps = mock_dependencies(&[]);

    let user = Addr::unchecked("user");

    let mut start_from_epoch = 1u64;
    let mut current_epoch_id = 10u64;

    // fill the lp_weight_history for the address with
    // [(1,2), (2,4), (3,6), (4,8), (5,10), (6,12), (7,14), (8,16), (9,18), (10,20)]
    for epoch in 1u64..=10u64 {
        let weight = Uint128::new(epoch as u128 * 2u128);
        ADDRESS_LP_WEIGHT_HISTORY
            .save(&mut deps.storage, (&user, epoch), &weight)
            .unwrap();
    }

    let weights =
        compute_user_weights(&deps.storage, &user, &start_from_epoch, &current_epoch_id).unwrap();
    assert_eq!(weights.len(), 11);

    for epoch in 1u64..=10u64 {
        assert_eq!(
            weights.get(&epoch).unwrap(),
            &Uint128::new(epoch as u128 * 2u128)
        );

        // reset the weight for epochs
        ADDRESS_LP_WEIGHT_HISTORY.remove(&mut deps.storage, (&user, epoch));
    }

    // fill the lp_weight_history for the address with
    // [(1,2), (5,10), (7,14)]
    for epoch in 1u64..=10u64 {
        if epoch % 2 == 0 || epoch % 3 == 0 {
            continue;
        }

        let weight = Uint128::new(epoch as u128 * 2u128);
        ADDRESS_LP_WEIGHT_HISTORY
            .save(&mut deps.storage, (&user, epoch), &weight)
            .unwrap();
    }

    // The result should be [(1,2), (5,10), (10,14)], with the skipped valued in between having the same
    // value as the previous, most recent value, i.e. epoch 2 3 4 having the value of 1 (latest weight seen in epoch 1)
    // then 5..7 having the value of 10 (latest weight seen in epoch 5)
    // then 8..=10 having the value of 14 (latest weight seen in epoch 7)
    let weights =
        compute_user_weights(&deps.storage, &user, &start_from_epoch, &current_epoch_id).unwrap();
    assert_eq!(weights.len(), 11);

    assert_eq!(weights.get(&1).unwrap(), &Uint128::new(2));
    assert_eq!(weights.get(&4).unwrap(), &Uint128::new(2));
    assert_eq!(weights.get(&5).unwrap(), &Uint128::new(10));
    assert_eq!(weights.get(&6).unwrap(), &Uint128::new(10));
    assert_eq!(weights.get(&7).unwrap(), &Uint128::new(14));
    assert_eq!(weights.get(&10).unwrap(), &Uint128::new(14));

    start_from_epoch = 6u64;
    let weights =
        compute_user_weights(&deps.storage, &user, &start_from_epoch, &current_epoch_id).unwrap();
    assert_eq!(weights.len(), 6);

    assert_eq!(weights.get(&5).unwrap(), &Uint128::new(10));
    assert_eq!(weights.get(&6).unwrap(), &Uint128::new(10));
    assert_eq!(weights.get(&7).unwrap(), &Uint128::new(14));
    assert_eq!(weights.get(&10).unwrap(), &Uint128::new(14));

    for epoch in 1u64..=10u64 {
        // reset the weight for epochs
        ADDRESS_LP_WEIGHT_HISTORY.remove(&mut deps.storage, (&user, epoch));
    }
}
