use cosmwasm_std::{coin, coins};

use crate::tests::suite::TestingSuite;
use crate::ContractError;

#[test]
fn test_bond_unsuccessful() {
    let mut suite = TestingSuite::default();
    let creator = suite.senders[0].clone();

    suite
        .instantiate_default()
        .bond(creator.clone(), &[], |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();

            match err {
                ContractError::PaymentError(error) => {
                    assert_eq!(error, cw_utils::PaymentError::NoFunds {})
                }
                _ => panic!("Wrong error type, should return ContractError::PaymentError"),
            }
        })
        .bond(
            creator.clone(),
            &coins(1_000u128, "non_whitelisted_asset"),
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::PaymentError(error) => {
                        assert_eq!(error, cw_utils::PaymentError::NoFunds {})
                    }
                    _ => panic!("Wrong error type, should return PaymentError::NoFunds"),
                }
            },
        )
        .bond(
            creator.clone(),
            &vec![coin(1_000u128, "bWHALE"), coin(1_000u128, "ampWHALE")],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::PaymentError(error) => {
                        assert_eq!(error, cw_utils::PaymentError::MultipleDenoms {})
                    }
                    _ => panic!("Wrong error type, should return PaymentError::MultipleDenoms"),
                }
            },
        );
}

#[test]
fn test_same_bond_multiple_times() {
    let mut suite = TestingSuite::default();
    let creator = suite.senders[0].clone();

    suite
        .instantiate_default()
        .add_one_day()
        .create_new_epoch()
        .bond(
            creator.clone(),
            &vec![coin(1_000u128, "bWHALE")],
            |result| {
                result.unwrap();
            },
        )
        .bond(
            creator.clone(),
            &vec![coin(2_000u128, "bWHALE")],
            |result| {
                result.unwrap();
            },
        )
        .query_bonded(Some(creator.clone().to_string()), |res| {
            assert_eq!(
                res.unwrap().1.bonded_assets,
                vec![coin(3_000u128, "bWHALE")]
            );
        });
}
