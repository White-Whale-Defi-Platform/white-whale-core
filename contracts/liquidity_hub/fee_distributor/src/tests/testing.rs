use std::fmt;

use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{coin, Addr, Timestamp, Uint128, Uint64};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use terraswap::asset::{Asset, AssetInfo};

use crate::msg::EpochConfig;
use crate::state::{Config, Epoch};
use crate::tests::epoch;
use crate::tests::robot::TestingRobot;
use crate::{helpers, ContractError};

#[test]
fn instantiate_successfully() {
    let mut robot = TestingRobot::new(mock_dependencies(), mock_env());
    let grace_period = Uint64::new(2);
    let epoch_config = EpochConfig {
        duration: Uint64::new(86400u64),           // a day
        genesis_epoch: Uint64::new(1678802400u64), // March 14, 2023 2:00:00 PM
    };

    robot
        .instantiate(
            mock_info("owner", &[]),
            "staking_contract_addr".to_string(),
            "fee_collector_addr".to_string(),
            grace_period,
            epoch_config.clone(),
        )
        .asset_config(Config {
            owner: Addr::unchecked("owner"),
            grace_period,
            staking_contract_addr: Addr::unchecked("staking_contract_addr"),
            fee_collector_addr: Addr::unchecked("fee_collector_addr"),
            epoch_config,
        });
}

#[test]
fn instantiate_unsuccessfully() {
    let mut robot = TestingRobot::new(mock_dependencies(), mock_env());
    let invalid_grace_period = Uint64::zero();
    let epoch_config = EpochConfig {
        duration: Uint64::new(86400u64),           // a day
        genesis_epoch: Uint64::new(1678802400u64), // March 14, 2023 2:00:00 PM
    };

    robot.instantiate_err(
        mock_info("owner", &[]),
        "staking_contract_addr".to_string(),
        "fee_collector_addr".to_string(),
        invalid_grace_period,
        epoch_config.clone(),
    );

    let invalid_grace_period = Uint64::new(11);
    robot.instantiate_err(
        mock_info("owner", &[]),
        "staking_contract_addr".to_string(),
        "fee_collector_addr".to_string(),
        invalid_grace_period,
        epoch_config.clone(),
    );

    let invalid_epoch_duration = Uint64::new(3600u64);
    robot.instantiate_err(
        mock_info("owner", &[]),
        "staking_contract_addr".to_string(),
        "fee_collector_addr".to_string(),
        Uint64::one(),
        epoch_config,
    );
}

#[test]
fn test_epoch_stuff() {
    let mut robot = TestingRobot::new(mock_dependencies(), mock_env());
    let grace_period = Uint64::new(10);
    let epoch_config = EpochConfig {
        duration: Uint64::new(86400u64),           // a day
        genesis_epoch: Uint64::new(1678802400u64), // March 14, 2023 2:00:00 PM
    };

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

    robot
        .instantiate(
            mock_info("owner", &[]),
            "staking_contract_addr".to_string(),
            "fee_collector_addr".to_string(),
            grace_period,
            epoch_config,
        )
        .create_new_epoch(mock_info("owner", &[]), vec![], |res| match res {
            _ => println!("res: {:?}", res),
        })
        .assert_current_epoch(&Epoch {
            id: 0,
            start_time: Default::default(),
            total: vec![],
            available: vec![],
            claimed: vec![],
        });
}

#[test]
fn test_create_new_epoch() {
    let mut robot = TestingRobot::new(mock_dependencies(), mock_env());
    let grace_period = Uint64::new(2);
    let epoch_config = EpochConfig {
        duration: Uint64::new(86400u64),           // a day
        genesis_epoch: Uint64::new(1678802400u64), // March 14, 2023 2:00:00 PM
    };

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
        start_time: Timestamp::from_seconds(1678986000),
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
            epoch_config,
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
            |_| {},
        )
        .assert_current_epoch(&expected_new_epoch)
        .assert_expiring_epoch(Some(&epoch::get_epochs()[1])) // make sure the second epoch is now expiring
        .create_new_epoch(mock_info("fee_collector_addr", &[]), vec![], |_| {})
        .query_epoch(5, |res| {
            let (r, epoch) = res.unwrap();
            r.assert_current_epoch(&epoch);
        });
}

#[test]
fn test_claimable_epochs() {
    let mut robot = TestingRobot::new(mock_dependencies(), mock_env());
    let grace_period = Uint64::new(2);
    let epoch_config = EpochConfig {
        duration: Uint64::new(86400u64),           // a day
        genesis_epoch: Uint64::new(1678802400u64), // March 14, 2023 2:00:00 PM
    };

    let epochs = epoch::get_epochs();
    let binding = epochs.clone();
    let claimable_epochs = binding
        .iter()
        .rev()
        .take(grace_period.u64() as usize)
        .collect::<Vec<_>>();

    robot
        .instantiate(
            mock_info("owner", &[]),
            "staking_contract_addr".to_string(),
            "fee_collector_addr".to_string(),
            grace_period,
            epoch_config,
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
    let grace_period = Uint64::new(2);
    let epoch_config = EpochConfig {
        duration: Uint64::new(86400u64),           // a day
        genesis_epoch: Uint64::new(1678802400u64), // March 14, 2023 2:00:00 PM
    };

    robot
        .instantiate(
            mock_info("owner", &[]),
            "staking_contract_addr".to_string(),
            "fee_collector_addr".to_string(),
            grace_period,
            epoch_config,
        )
        .query_current_epoch(|res| {
            let epoch = res.unwrap();
            assert_eq!(epoch, Epoch::default());
        })
        .query_epoch(10, |res| {
            // epoch 10 doesn't exist, it should return the default value
            let (_, epoch) = res.unwrap();
            assert_eq!(epoch, Epoch::default());
        });
}

#[test]
fn test_update_config() {
    let mut robot = TestingRobot::new(mock_dependencies(), mock_env());
    let grace_period = Uint64::new(2);
    let epoch_config = EpochConfig {
        duration: Uint64::new(86400u64),           // a day
        genesis_epoch: Uint64::new(1678802400u64), // March 14, 2023 2:00:00 PM
    };

    // let new_config = Config {
    //     owner: Addr::unchecked("new_owner"),
    //     staking_contract_addr: Addr::unchecked("new_staking_contract_addr"),
    //     fee_collector_addr: Addr::unchecked("new_fee_collector_addr"),
    //     grace_period: Uint64::new(3),
    //     epoch_duration: Uint64::new(86400u64),
    // };
    let new_config = Config {
        owner: Addr::unchecked("new_owner"),
        staking_contract_addr: Addr::unchecked("new_staking_contract_addr"),
        fee_collector_addr: Addr::unchecked("new_fee_collector_addr"),
        grace_period: Uint64::new(3),
        epoch_config: EpochConfig {
            duration: Default::default(),
            genesis_epoch: Default::default(),
        },
    };

    robot
        .instantiate(
            mock_info("owner", &[]),
            "staking_contract_addr".to_string(),
            "fee_collector_addr".to_string(),
            grace_period,
            epoch_config,
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
