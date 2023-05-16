use anyhow::Error;
use cosmwasm_std::{coins, Addr, Uint128};
use std::cell::RefCell;

use white_whale::pool_network::asset::{Asset, AssetInfo};

use crate::tests::suite::TestingSuite;

#[test]
fn instantiate_incentive_factory_successful() {
    let mut suite = TestingSuite::default();

    suite.instantiate(
        "fee_collector_addr".to_string(),
        Asset {
            amount: Uint128::new(1_000u128),
            info: AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
        },
        1_000,
        1,
        1_000,
        1_000,
        2_000,
    );
}

#[test]
fn instantiate_incentive_factory_unsuccessful() {
    let mut suite = TestingSuite::default();

    suite.instantiate_err(
        "fee_collector_addr".to_string(),
        Asset {
            amount: Uint128::new(1_000u128),
            info: AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
        },
        1_000,
        1,
        1_000,
        1_000,
        500,
        |error| {
            let err = error
                .downcast::<incentive_factory::error::ContractError>()
                .unwrap();

            match err {
                incentive_factory::error::ContractError::InvalidUnbondingRange { min, max } => {
                    assert_eq!(min, 1000);
                    assert_eq!(max, 500);
                }
                _ => panic!("Wrong error type, should return ContractError::InvalidUnbondingRange"),
            }
        },
    );
}

#[test]
fn create_incentive() {
    let mut suite =
        TestingSuite::default_with_balances(coins(1_000_000_000u128, "uwhale".to_string()));
    let creator = suite.creator();
    let unauthorized = suite.senders[2].clone();

    suite.instantiate_default().create_lp_tokens();

    let lp_address_1 = AssetInfo::Token {
        contract_addr: suite.cw20_tokens.first().unwrap().to_string(),
    };
    let lp_address_2 = AssetInfo::Token {
        contract_addr: suite.cw20_tokens.last().unwrap().to_string(),
    };

    // first try to execute anything on the incentive factory contract from a non-owner, it should error
    // then do it with the owner of the contract
    suite
        .create_incentive(unauthorized, lp_address_1.clone(), |result| {
            let err = result
                .unwrap_err()
                .downcast::<incentive_factory::error::ContractError>()
                .unwrap();

            match err {
                incentive_factory::error::ContractError::Unauthorized {} => {}
                _ => panic!("Wrong error type, should return ContractError::Unauthorized"),
            }
        })
        .create_incentive(creator.clone(), lp_address_1.clone(), |result| {
            result.unwrap();
        })
        .query_incentive(lp_address_1.clone(), |result| {
            let incentive = result.unwrap();
            assert!(incentive.is_some());
        })
        // this should error cuz the incentive for that lp was already created
        .create_incentive(creator.clone(), lp_address_1.clone(), |result| {
            let err = result
                .unwrap_err()
                .downcast::<incentive_factory::error::ContractError>()
                .unwrap();

            match err {
                incentive_factory::error::ContractError::DuplicateIncentiveContract { .. } => {}
                _ => panic!(
                    "Wrong error type, should return ContractError::DuplicateIncentiveContract"
                ),
            }
        })
        .create_incentive(creator.clone(), lp_address_2, |result| {
            result.unwrap();
        })
        .query_incentives(None, None, |result| {
            let incentives = result.unwrap();
            assert_eq!(incentives.len(), 2usize);
        });
}

#[test]
fn open_close_positions_from_incentive() {
    let mut suite =
        TestingSuite::default_with_balances(coins(1_000_000_000u128, "uwhale".to_string()));
    let alice = suite.creator();
    let bob = suite.senders[1].clone();
    let carol = suite.senders[2].clone();

    suite.instantiate_default().create_lp_tokens();

    let lp_address_1 = AssetInfo::Token {
        contract_addr: suite.cw20_tokens.first().unwrap().to_string(),
    };

    let mut incentive_addr = RefCell::new(Addr::unchecked(""));

    suite
        .create_incentive(alice.clone(), lp_address_1.clone(), |result| {
            result.unwrap();
        })
        .query_incentive(lp_address_1.clone(), |result| {
            let incentive = result.unwrap();
            assert!(incentive.is_some());
            *incentive_addr.borrow_mut() = incentive.unwrap();
        });
}
