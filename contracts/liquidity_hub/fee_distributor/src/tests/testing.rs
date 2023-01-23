use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{coin, Addr, Coin, Response, Uint128};

use terraswap::asset::{Asset, AssetInfo};

use crate::state::{Config, Epoch};
use crate::tests::epoch;
use crate::tests::robot::TestingRobot;
use crate::{helpers, ContractError};

#[test]
fn instantiate_successfully() {
    let mut robot = TestingRobot::new(mock_dependencies(), mock_env());
    let grace_period = 2;

    robot
        .instantiate(
            mock_info("owner", &[]),
            "staking_contract_addr".to_string(),
            "fee_collector_addr".to_string(),
            grace_period,
        )
        .asset_config(Config {
            owner: Addr::unchecked("owner"),
            grace_period,
            staking_contract_addr: Addr::unchecked("staking_contract_addr"),
            fee_collector_addr: Addr::unchecked("fee_collector_addr"),
        });
}

#[test]
fn instantiate_unsuccessfully() {
    let mut robot = TestingRobot::new(mock_dependencies(), mock_env());
    let invalid_grace_period = 0;

    robot.instantiate_err(
        mock_info("owner", &[]),
        "staking_contract_addr".to_string(),
        "fee_collector_addr".to_string(),
        invalid_grace_period,
    );

    let invalid_grace_period = 11;
    robot.instantiate_err(
        mock_info("owner", &[]),
        "staking_contract_addr".to_string(),
        "fee_collector_addr".to_string(),
        invalid_grace_period,
    );
}

#[test]
fn test_create_new_epoch() {
    let mut robot = TestingRobot::new(mock_dependencies(), mock_env());
    let grace_period = 2;

    let coins = vec![coin(5_000u128, "uwhale"), coin(1_000u128, "uatom")];
    let fees = vec![
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            amount: Uint128::from(5_000u128),
        },
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uatom".to_string(),
            },
            amount: Uint128::from(1_000u128),
        },
    ];

    let expiring_epoch = epoch::get_epochs().first().cloned().unwrap();
    let total_fees = helpers::aggregate_fees(fees.clone(), expiring_epoch.available.clone());

    let expected_new_epoch = Epoch {
        id: 4,
        total: total_fees.clone(),
        available: total_fees.clone(),
        claimed: vec![],
    };

    robot
        .instantiate(
            mock_info("owner", &[]),
            "staking_contract_addr".to_string(),
            "fee_collector_addr".to_string(),
            grace_period,
        )
        .add_epochs_to_state(epoch::get_epochs())
        .assert_current_epoch(epoch::get_epochs().last().unwrap())
        .assert_expiring_epoch(Some(&expiring_epoch))
        .create_new_epoch(mock_info("unauthorized", &[]), vec![], |res| match res {
            Ok(_) => panic!("should have returned ContractError::Unauthorized"),
            Err(ContractError::Unauthorized {}) => (),
            _ => panic!("should have returned ContractError::Unauthorized"),
        })
        .create_new_epoch(
            mock_info("fee_collector_addr", &[coin(100u128, "uwhale")]),
            vec![],
            |res| match res {
                Ok(_) => panic!("should have returned ContractError::AssetMismatch"),
                Err(ContractError::AssetMismatch {}) => (),
                _ => panic!("should have returned ContractError::AssetMismatch"),
            },
        )
        .create_new_epoch(
            mock_info("fee_collector_addr", &coins),
            fees.clone(),
            |res| {},
        )
        .assert_current_epoch(&expected_new_epoch)
        .assert_expiring_epoch(Some(&epoch::get_epochs()[1])) // make sure the second epoch is now expiring
        .create_new_epoch(mock_info("fee_collector_addr", &[]), vec![], |res| {})
        .query_epoch(5, |res| {
            let (r, epoch) = res.unwrap();
            r.assert_current_epoch(&epoch);
        });
}

#[test]
fn test_claimable_epochs() {
    let mut robot = TestingRobot::new(mock_dependencies(), mock_env());
    let grace_period = 2u128;

    let epochs = epoch::get_epochs();
    let binding = epochs.clone();
    let claimable_epochs = binding
        .iter()
        .rev()
        .take(grace_period as usize)
        .collect::<Vec<_>>();

    robot
        .instantiate(
            mock_info("owner", &[]),
            "staking_contract_addr".to_string(),
            "fee_collector_addr".to_string(),
            grace_period,
        )
        .add_epochs_to_state(epochs)
        .query_claimable_epochs(|res| {
            let (_, epochs) = res.unwrap();

            assert_eq!(epochs.len(), claimable_epochs.len());
            for (e, a) in epochs.iter().zip(claimable_epochs.iter()) {
                assert_eq!(e, *a);
            }
        });
}

#[test]
fn test_current_epoch_no_epochs() {
    let mut robot = TestingRobot::new(mock_dependencies(), mock_env());
    let grace_period = 2u128;

    robot
        .instantiate(
            mock_info("owner", &[]),
            "staking_contract_addr".to_string(),
            "fee_collector_addr".to_string(),
            grace_period,
        )
        .query_current_epoch(|res| {
            let epoch = res.unwrap();
            assert_eq!(epoch, Epoch::default());
        });
}

#[test]
fn test_update_config() {
    let mut robot = TestingRobot::new(mock_dependencies(), mock_env());
    let grace_period = 2u128;

    let new_config = Config {
        owner: Addr::unchecked("new_owner"),
        staking_contract_addr: Addr::unchecked("new_staking_contract_addr"),
        fee_collector_addr: Addr::unchecked("new_fee_collector_addr"),
        grace_period: 3u128,
    };

    robot
        .instantiate(
            mock_info("owner", &[]),
            "staking_contract_addr".to_string(),
            "fee_collector_addr".to_string(),
            grace_period,
        )
        .update_config(
            mock_info("unauthorized", &[]),
            new_config.clone(),
            |res| match res {
                Ok(_) => panic!("should have returned ContractError::Unauthorized"),
                Err(ContractError::Unauthorized {}) => (),
                _ => panic!("should have returned ContractError::Unauthorized"),
            },
        )
        .update_config(
            mock_info("owner", &[]),
            Config {
                grace_period: 0u128,
                ..new_config.clone()
            },
            |res| match res {
                Ok(_) => panic!("should have returned ContractError::InvalidGracePeriod"),
                Err(ContractError::InvalidGracePeriod { .. }) => (),
                _ => panic!("should have returned ContractError::InvalidGracePeriod"),
            },
        )
        .update_config(
            mock_info("owner", &[]),
            Config {
                grace_period: 11u128,
                ..new_config.clone()
            },
            |res| match res {
                Ok(_) => panic!("should have returned ContractError::InvalidGracePeriod"),
                Err(ContractError::InvalidGracePeriod { .. }) => (),
                _ => panic!("should have returned ContractError::InvalidGracePeriod"),
            },
        )
        .update_config(mock_info("owner", &[]), new_config.clone(), |res| {})
        .asset_config(new_config.clone());
}
