extern crate core;

use std::cell::RefCell;

use cosmwasm_std::{coin, Addr, Decimal, Uint128};

use vault_manager::ContractError;
use white_whale::fee::Fee;
use white_whale::pool_network::asset::{Asset, AssetInfo};
use white_whale::vault_manager::{FilterVaultBy, LpTokenType, VaultFee};

use crate::common::suite::TestingSuite;
use crate::common::MOCK_CONTRACT_ADDR;

mod common;

#[test]
fn instantiate_vault_manager_successful() {
    let mut suite = TestingSuite::default_with_balances(vec![]);

    suite.instantiate(
        MOCK_CONTRACT_ADDR.to_string(),
        LpTokenType::TokenFactory,
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            amount: Uint128::new(1_000u128),
        },
    );

    let cw20_code_id = suite.create_cw20_token();
    suite.instantiate(
        MOCK_CONTRACT_ADDR.to_string(),
        LpTokenType::Cw20(cw20_code_id),
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            amount: Uint128::new(1_000u128),
        },
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

#[test]
fn create_vaults() {
    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_000u128, "uwhale".to_string()),
        coin(1_000_000_000u128, "factory/creator/uLP".to_string()),
        coin(1_000_000_000u128, "factory/another_creator/uLP".to_string()),
    ]);

    let creator = suite.creator();
    let other = suite.senders[1].clone();

    suite
        .instantiate_with_cw20_lp_token()
        .create_vault(
            creator.clone(),
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            None,
            VaultFee {
                protocol_fee: Fee {
                    share: Decimal::from_ratio(1u128, 2000u128),
                },
                flash_loan_fee: Fee {
                    share: Decimal::from_ratio(1u128, 1000u128),
                },
            },
            vec![coin(900u128, "uwhale".to_string())],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::InvalidVaultCreationFee { .. } => {}
                    _ => panic!(
                        "Wrong error type, should return ContractError::InvalidVaultCreationFee"
                    ),
                }
            },
        )
        .create_vault(
            creator,
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            None,
            VaultFee {
                protocol_fee: Fee {
                    share: Decimal::from_ratio(1u128, 2000u128),
                },
                flash_loan_fee: Fee {
                    share: Decimal::from_ratio(1u128, 1000u128),
                },
            },
            vec![coin(1_000u128, "uwhale".to_string())],
            |result| {
                result.unwrap();
            },
        )
        .create_vault(
            other.clone(),
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            Some("cheaper_vault".to_string()),
            VaultFee {
                protocol_fee: Fee {
                    share: Decimal::from_ratio(1u128, 20000u128),
                },
                flash_loan_fee: Fee {
                    share: Decimal::from_ratio(1u128, 10000u128),
                },
            },
            vec![coin(1_000u128, "uwhale".to_string())],
            |result| {
                result.unwrap();
            },
        )
        .create_vault(
            other,
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            Some("cheaper_vault".to_string()),
            VaultFee {
                protocol_fee: Fee {
                    share: Decimal::from_ratio(1u128, 20000u128),
                },
                flash_loan_fee: Fee {
                    share: Decimal::from_ratio(1u128, 10000u128),
                },
            },
            vec![coin(1_000u128, "uwhale".to_string())],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::ExistingVault { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::ExistingVault"),
                }
            },
        )
        .query_vaults(None, None, |result| {
            let vaults_response = result.unwrap();

            assert_eq!(vaults_response.vaults.len(), 2);
            assert_eq!(vaults_response.vaults[0].identifier, "0".to_string());
            assert_eq!(
                vaults_response.vaults[1].identifier,
                "cheaper_vault".to_string()
            );
        });
}

#[test]
fn deposit_withdraw() {
    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_000u128, "uwhale".to_string()),
        coin(1_000_000_000u128, "uluna".to_string()),
        coin(1_000_000_000u128, "factory/creator/uLP".to_string()),
        coin(1_000_000_000u128, "factory/another_creator/uLP".to_string()),
    ]);

    let creator = suite.creator();
    let other = suite.senders[1].clone();
    let yet_another = suite.senders[2].clone();

    let vault_lp_addr = RefCell::new(AssetInfo::Token {
        contract_addr: "".to_string(),
    });

    suite
        .instantiate_with_cw20_lp_token()
        .create_vault(
            creator.clone(),
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            None,
            VaultFee {
                protocol_fee: Fee {
                    share: Decimal::from_ratio(1u128, 2000u128),
                },
                flash_loan_fee: Fee {
                    share: Decimal::from_ratio(1u128, 1000u128),
                },
            },
            vec![coin(1_000u128, "uwhale".to_string())],
            |result| {
                result.unwrap();
            },
        )
        .update_config(
            creator.clone(),
            None,
            None,
            None,
            None,
            Some(false),
            Some(false),
            vec![],
            |result| {
                result.unwrap();
            },
        )
        .deposit(
            creator.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                amount: Uint128::new(5_000u128),
            },
            "0".to_string(),
            vec![coin(5_000u128, "uluna".to_string())],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::Unauthorized { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::Unauthorized"),
                }
            },
        )
        .update_config(
            creator.clone(),
            None,
            None,
            None,
            None,
            Some(true),
            None,
            vec![],
            |result| {
                result.unwrap();
            },
        )
        .deposit(
            creator.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                amount: Uint128::new(5_000u128),
            },
            "unexisting_vault".to_string(),
            vec![coin(5_000u128, "uluna".to_string())],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::NonExistentVault { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::NonExistentVault"),
                }
            },
        )
        .deposit(
            creator.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                amount: Uint128::new(5_000u128),
            },
            "0".to_string(),
            vec![coin(5_000u128, "uluna".to_string())],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::AssetMismatch { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::AssetMismatch"),
                }
            },
        )
        .deposit(
            creator.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(5_000u128),
            },
            "0".to_string(),
            vec![coin(3_000u128, "uwhale".to_string())],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::FundsMismatch { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::FundsMismatch"),
                }
            },
        )
        .deposit(
            creator.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(5_000u128),
            },
            "0".to_string(),
            vec![coin(5_000u128, "uwhale".to_string())],
            |result| {
                result.unwrap();
            },
        )
        .query_vault(FilterVaultBy::Identifier("0".to_string()), |result| {
            let vault_response = result.unwrap();
            let vault = vault_response.vaults.get(0).unwrap();
            *vault_lp_addr.borrow_mut() = vault.lp_asset.clone();

            println!("vault: {:?}", vault);
            assert_eq!(
                vault.asset,
                Asset {
                    amount: Uint128::new(5_000u128),
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string()
                    },
                }
            );
        });

    let vault_manager = suite.vault_manager_addr.clone();
    let random_cw20_token = suite.cw20_tokens.get(0).unwrap().clone();

    println!("vault_lp_addr: {:?}", vault_lp_addr.borrow().clone());

    suite
        .query_balance(
            vault_lp_addr.clone().into_inner(),
            creator.clone(),
            |result| {
                // 4k to the user
                assert_eq!(result, Uint128::new(4_000u128));
            },
        )
        .query_balance(
            vault_lp_addr.clone().into_inner(),
            vault_manager.clone(),
            |result| {
                //  1k in vault
                assert_eq!(result, Uint128::new(1_000u128));
            },
        )
        .deposit(
            other.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(5_000u128),
            },
            "0".to_string(),
            vec![coin(5_000u128, "uwhale".to_string())],
            |result| {
                result.unwrap();
            },
        )
        .query_balance(
            vault_lp_addr.clone().into_inner(),
            other.clone(),
            |result| {
                assert_eq!(result, Uint128::new(5_000u128));
            },
        )
        .withdraw(
            other.clone(),
            Asset {
                info: vault_lp_addr.clone().into_inner(),
                amount: Uint128::new(2_000u128),
            },
            vec![],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::Unauthorized { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::Unauthorized"),
                }
            },
        )
        .update_config(
            creator.clone(),
            None,
            None,
            None,
            None,
            None,
            Some(true),
            vec![],
            |result| {
                result.unwrap();
            },
        )
        .withdraw(
            other.clone(),
            Asset {
                info: AssetInfo::Token {
                    contract_addr: random_cw20_token.to_string(),
                },
                amount: Uint128::new(2_000u128),
            },
            vec![],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::NonExistentVault { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::NonExistentVault"),
                }
            },
        )
        .withdraw(
            other.clone(),
            Asset {
                info: vault_lp_addr.clone().into_inner(),
                amount: Uint128::new(2_000u128),
            },
            vec![],
            |result| {
                result.unwrap();
            },
        )
        .query_balance(
            vault_lp_addr.clone().into_inner(),
            other.clone(),
            |result| {
                assert_eq!(result, Uint128::new(3_000u128));
            },
        )
        .withdraw(
            creator.clone(),
            Asset {
                info: vault_lp_addr.clone().into_inner(),
                amount: Uint128::new(4_000u128),
            },
            vec![],
            |result| {
                result.unwrap();
            },
        )
        .query_balance(
            vault_lp_addr.clone().into_inner(),
            creator.clone(),
            |result| {
                assert_eq!(result, Uint128::zero());
            },
        );
}
