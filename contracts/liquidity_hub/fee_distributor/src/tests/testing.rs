use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{Addr, Uint64};

use white_whale_std::fee_distributor::Config;

use crate::tests::robot::TestingRobot;
use crate::ContractError;
use white_whale_std::epoch_manager::epoch_manager::EpochConfig;
use white_whale_std::pool_network::asset::AssetInfo;

#[test]
fn instantiate_successfully() {
    let mut robot = TestingRobot::new(mock_dependencies(), mock_env());
    let grace_period = Uint64::new(2);
    let distribution_asset = AssetInfo::NativeToken {
        denom: "uwhale".to_string(),
    };
    let epoch_config = EpochConfig {
        duration: Uint64::new(86_400_000_000_000u64), // a day
        genesis_epoch: Uint64::new(1_678_802_400_000_000_000_u64), // March 14, 2023 2:00:00 PM
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
        duration: Uint64::new(86_400_000_000_000u64), // a day
        genesis_epoch: Uint64::new(1_678_802_400_000_000_000_u64), // March 14, 2023 2:00:00 PM
    };

    robot.instantiate_err(
        mock_info("owner", &[]),
        "bonding_contract_addr".to_string(),
        "fee_collector_addr".to_string(),
        invalid_grace_period,
        epoch_config.clone(),
        distribution_asset.clone(),
    );

    let invalid_grace_period = Uint64::new(31);
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
            duration: invalid_epoch_duration,                          // a day
            genesis_epoch: Uint64::new(1_678_802_400_000_000_000_u64), // March 14, 2023 2:00:00 PM
        },
        distribution_asset.clone(),
    );
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
            duration: Uint64::new(86_400_000_000_000u64), // a day
            genesis_epoch: Uint64::new(1_678_802_400_000_000_000_u64), // March 14, 2023 2:00:00 PM
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
                grace_period: Uint64::new(31),
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
                grace_period: Uint64::new(1),
                ..new_config.clone()
            },
            |res| match res {
                Ok(_) => panic!("should have returned ContractError::GracePeriodDecrease"),
                Err(ContractError::GracePeriodDecrease {}) => (),
                _ => panic!("should have returned ContractError::GracePeriodDecrease"),
            },
        )
        .update_config(mock_info("owner", &[]), new_config.clone(), |_| {})
        .asset_config(new_config.clone());
}
