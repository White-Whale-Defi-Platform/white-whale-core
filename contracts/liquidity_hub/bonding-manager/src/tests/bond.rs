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

#[test]
fn test_bonding_unbonding_without_creating_new_epoch_on_time() {
    let mut suite = TestingSuite::default();
    let creator = suite.senders[0].clone();

    suite
        .instantiate_default()
        .add_one_day()
        .create_new_epoch()
        .fast_forward(86_399)
        // bonds the last second before the new epoch kicks in
        .bond(
            creator.clone(),
            &vec![coin(1_000u128, "bWHALE")],
            |result| {
                result.unwrap();
            },
        )
        .fast_forward(1)
        // tries to bond when the new epoch should have been created, but since it wasn't it is triggered
        // by the contract via a submsg/reply
        .query_current_epoch(|res| {
            assert_eq!(res.unwrap().1.epoch.id, 1u64);
        })
        .bond(
            creator.clone(),
            &vec![coin(2_000u128, "bWHALE")],
            |result| {
                result.unwrap();
            },
        )
        .query_current_epoch(|res| {
            assert_eq!(res.unwrap().1.epoch.id, 2u64);
        })
        .query_bonded(Some(creator.clone().to_string()), |res| {
            assert_eq!(
                res.unwrap().1.bonded_assets,
                vec![coin(3_000u128, "bWHALE")]
            );
        });

    // now try unbonding

    suite
        .add_one_day()
        .fast_forward(60) //one minute past when the new epoch should have been created
        .query_current_epoch(|res| {
            assert_eq!(res.unwrap().1.epoch.id, 2u64);
        })
        .unbond(creator.clone(), coin(3_000u128, "bWHALE"), |result| {
            result.unwrap();
        })
        .query_current_epoch(|res| {
            assert_eq!(res.unwrap().1.epoch.id, 3u64);
        });
}
