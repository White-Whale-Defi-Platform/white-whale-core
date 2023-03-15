use std::fmt;

use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{coin, Addr, Timestamp, Uint128, Uint64};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use white_whale::fee_distributor::{Config, Epoch};

use crate::tests::epoch;
use crate::tests::robot::TestingRobot;
use crate::{helpers, ContractError};
use white_whale::fee_distributor::EpochConfig;
use white_whale::pool_network::asset::{Asset, AssetInfo};

#[test]
fn instantiate_successfully() {
    let mut robot = TestingRobot::new(mock_dependencies(), mock_env());
    let grace_period = Uint64::new(2);
    let distribution_asset = AssetInfo::NativeToken {
        denom: "uwhale".to_string(),
    };
    let epoch_config = EpochConfig {
        duration: Uint64::new(86400u64),           // a day
        genesis_epoch: Uint64::new(1678802400u64), // March 14, 2023 2:00:00 PM
    };

    robot
        .instantiate(
            mock_info("owner", &[]),
            "bonding_contract_addr".to_string(),
            "fee_collector_addr".to_string(),
            grace_period,
            epoch_config.clone(),
            distribution_asset.clone(),
        )
        .asset_config(Config {
            owner: Addr::unchecked("owner"),
            grace_period,
            bonding_contract_addr: Addr::unchecked("bonding_contract_addr"),
            fee_collector_addr: Addr::unchecked("fee_collector_addr"),
            epoch_config,
            distribution_asset,
        });
}

#[test]
fn instantiate_unsuccessfully() {
    let mut robot = TestingRobot::new(mock_dependencies(), mock_env());
    let invalid_grace_period = Uint64::zero();
    let distribution_asset = AssetInfo::NativeToken {
        denom: "uwhale".to_string(),
    };
    let epoch_config = EpochConfig {
        duration: Uint64::new(86400u64),           // a day
        genesis_epoch: Uint64::new(1678802400u64), // March 14, 2023 2:00:00 PM
    };

    robot.instantiate_err(
        mock_info("owner", &[]),
        "bonding_contract_addr".to_string(),
        "fee_collector_addr".to_string(),
        invalid_grace_period,
        epoch_config.clone(),
        distribution_asset.clone(),
    );

    let invalid_grace_period = Uint64::new(11);
    robot.instantiate_err(
        mock_info("owner", &[]),
        "bonding_contract_addr".to_string(),
        "fee_collector_addr".to_string(),
        invalid_grace_period,
        epoch_config.clone(),
        distribution_asset.clone(),
    );

    let invalid_epoch_duration = Uint64::new(3600u64);
    robot.instantiate_err(
        mock_info("owner", &[]),
        "bonding_contract_addr".to_string(),
        "fee_collector_addr".to_string(),
        Uint64::one(),
        EpochConfig {
            duration: invalid_epoch_duration,          // a day
            genesis_epoch: Uint64::new(1678802400u64), // March 14, 2023 2:00:00 PM
        },
        distribution_asset.clone(),
    );
}

#[test]
fn test_create_new_epoch() {
    let mut robot = TestingRobot::new(mock_dependencies(), mock_env());

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
        id: Uint64::new(4),
        start_time: Timestamp::from_seconds(1678986000),
        total: total_fees.clone(),
        available: total_fees.clone(),
        claimed: vec![],
    };

    robot
        .instantiate_default()
        .add_epochs_to_state(epoch::get_epochs())
        .assert_current_epoch(epoch::get_epochs().last().unwrap())
        .assert_expiring_epoch(Some(&expiring_epoch))
        .create_new_epoch(mock_info("unauthorized", &[]), |res| match res {
            Ok(_) => panic!("should have returned ContractError::Unauthorized"),
            Err(ContractError::Unauthorized {}) => (),
            _ => panic!("should have returned ContractError::Unauthorized"),
        })
        .create_new_epoch(
            mock_info("fee_collector_addr", &[coin(100u128, "uwhale")]),
            |res| match res {
                Ok(_) => panic!("should have returned ContractError::AssetMismatch"),
                Err(ContractError::AssetMismatch {}) => (),
                _ => panic!("should have returned ContractError::AssetMismatch"),
            },
        )
        .create_new_epoch(mock_info("fee_collector_addr", &coins), |_| {})
        .assert_current_epoch(&expected_new_epoch)
        .assert_expiring_epoch(Some(&epoch::get_epochs()[1])) // make sure the second epoch is now expiring
        .create_new_epoch(mock_info("fee_collector_addr", &[]), |_| {})
        .query_epoch(Uint64::new(5), |res| {
            let (r, epoch) = res.unwrap();
            r.assert_current_epoch(&epoch);
        });
}

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

#[test]
fn test_current_epoch_no_epochs() {
    let mut robot = TestingRobot::new(mock_dependencies(), mock_env());

    robot
        .instantiate_default()
        .query_current_epoch(|res| {
            let epoch = res.unwrap();
            assert_eq!(epoch, Epoch::default());
        })
        .query_epoch(Uint64::new(10), |res| {
            // epoch 10 doesn't exist, it should return the default value
            let (_, epoch) = res.unwrap();
            assert_eq!(epoch, Epoch::default());
        });
}

#[test]
fn test_update_config() {
    let mut robot = TestingRobot::new(mock_dependencies(), mock_env());

    let new_config = Config {
        owner: Addr::unchecked("new_owner"),
        bonding_contract_addr: Addr::unchecked("new_bonding_contract_addr"),
        fee_collector_addr: Addr::unchecked("new_fee_collector_addr"),
        grace_period: Uint64::new(3),
        epoch_config: EpochConfig {
            duration: Uint64::new(86400u64),           // a day
            genesis_epoch: Uint64::new(1678802400u64), // March 14, 2023 2:00:00 PM
        },
        distribution_asset: AssetInfo::NativeToken {
            denom: "uwhale".to_string(),
        },
    };

    robot
        .instantiate_default()
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
                grace_period: Uint64::zero(),
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
                grace_period: Uint64::new(11),
                ..new_config.clone()
            },
            |res| match res {
                Ok(_) => panic!("should have returned ContractError::InvalidGracePeriod"),
                Err(ContractError::InvalidGracePeriod { .. }) => (),
                _ => panic!("should have returned ContractError::InvalidGracePeriod"),
            },
        )
        .update_config(mock_info("owner", &[]), new_config.clone(), |_| {})
        .asset_config(new_config.clone());
}
