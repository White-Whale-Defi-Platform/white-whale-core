use cosmwasm_std::{coin, Addr, Decimal, Uint128};

use vault_manager::ContractError;
use white_whale::fee::Fee;
use white_whale::pool_network::asset::{Asset, AssetInfo};
use white_whale::vault_manager::{LpTokenType, VaultFee};

use crate::common::suite::TestingSuite;

mod common;

#[test]
fn instantiate_vault_manager_successful() {
    let mut suite = TestingSuite::default();

    suite.instantiate(
        "whale_lair_addr".to_string(),
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
        "whale_lair_addr".to_string(),
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
    let mut suite = TestingSuite::default();
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

#[cfg(any(feature = "token_factory", feature = "osmosis_token_factory"))]
#[test]
fn create_remove_vault() {
    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_000u128, "uwhale".to_string()),
        coin(1_000_000_000u128, "factory/creator/uLP".to_string()),
        coin(1_000_000_000u128, "factory/another_creator/uLP".to_string()),
    ]);

    let creator = suite.creator();
    let other = suite.senders[1].clone();
    let unauthorized = suite.senders[2].clone();

    suite.instantiate_with_cw20_lp_token().create_vault(
        other,
        AssetInfo::NativeToken {
            denom: "uwhale".to_string(),
        },
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
    );
}
