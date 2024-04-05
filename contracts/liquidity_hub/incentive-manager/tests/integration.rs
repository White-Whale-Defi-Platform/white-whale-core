extern crate core;

use std::cell::RefCell;

use cosmwasm_std::{
    coin, coins, to_json_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal, Uint128, WasmMsg,
};
use cw_ownable::OwnershipError;
use incentive_manager::ContractError;

use white_whale_std::fee::Fee;
use white_whale_std::pool_network::asset::{Asset, AssetInfo, PairType};
use white_whale_std::pool_network::pair::PoolFee;
use white_whale_std::vault_manager::{
    AssetQueryParams, FilterVaultBy, PaybackAssetResponse, VaultFee,
};

use crate::common::suite::TestingSuite;
use crate::common::MOCK_CONTRACT_ADDR;

mod common;

#[test]
fn instantiate_incentive_manager() {
    let mut suite =
        TestingSuite::default_with_balances(vec![coin(1_000_000_000u128, "uwhale".to_string())]);

    suite.instantiate_err(
        MOCK_CONTRACT_ADDR.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        Coin {
            denom: "uwhale".to_string(),
            amount: Uint128::new(1_000u128),
        },
        0,
        14,
        86_400,
        31_536_000,
        Decimal::percent(10),
        |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();

            match err {
                ContractError::UnspecifiedConcurrentIncentives { .. } => {}
                _ => panic!("Wrong error type, should return ContractError::UnspecifiedConcurrentIncentives"),
            }
        }
    ).instantiate_err(
        MOCK_CONTRACT_ADDR.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        Coin {
            denom: "uwhale".to_string(),
            amount: Uint128::new(1_000u128),
        },
        1,
        14,
        86_400,
        86_399,
        Decimal::percent(10),
        |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();

            match err {
                ContractError::InvalidUnbondingRange { .. } => {}
                _ => panic!("Wrong error type, should return ContractError::InvalidUnbondingRange"),
            }
        }
    ).instantiate_err(
        MOCK_CONTRACT_ADDR.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        Coin {
            denom: "uwhale".to_string(),
            amount: Uint128::new(1_000u128),
        },
        1,
        14,
        86_400,
        86_500,
        Decimal::percent(101),
        |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();

            match err {
                ContractError::InvalidEmergencyUnlockPenalty { .. } => {}
                _ => panic!("Wrong error type, should return ContractError::InvalidEmergencyUnlockPenalty"),
            }
        }
    ).instantiate(
        MOCK_CONTRACT_ADDR.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        Coin {
            denom: "uwhale".to_string(),
            amount: Uint128::new(1_000u128),
        },
        7,
        14,
        86_400,
        31_536_000,
        Decimal::percent(10), //10% penalty
    );
}

#[test]
fn verify_ownership() {
    let mut suite = TestingSuite::default_with_balances(vec![]);
    let creator = suite.creator();
    let other = suite.senders[1].clone();
    let unauthorized = suite.senders[2].clone();

    suite
        .instantiate_default()
        .query_ownership(|result| {
            let ownership = result.unwrap();
            assert_eq!(Addr::unchecked(ownership.owner.unwrap()), creator);
        })
        .update_ownership(
            unauthorized,
            cw_ownable::Action::TransferOwnership {
                new_owner: other.to_string(),
                expiry: None,
            },
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::OwnershipError { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::OwnershipError"),
                }
            },
        )
        .update_ownership(
            creator,
            cw_ownable::Action::TransferOwnership {
                new_owner: other.to_string(),
                expiry: None,
            },
            |result| {
                result.unwrap();
            },
        )
        .update_ownership(
            other.clone(),
            cw_ownable::Action::AcceptOwnership,
            |result| {
                result.unwrap();
            },
        )
        .query_ownership(|result| {
            let ownership = result.unwrap();
            assert_eq!(Addr::unchecked(ownership.owner.unwrap()), other);
        })
        .update_ownership(
            other.clone(),
            cw_ownable::Action::RenounceOwnership,
            |result| {
                result.unwrap();
            },
        )
        .query_ownership(|result| {
            let ownership = result.unwrap();
            assert!(ownership.owner.is_none());
        });
}
