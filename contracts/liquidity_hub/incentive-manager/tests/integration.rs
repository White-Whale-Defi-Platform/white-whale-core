extern crate core;

use cosmwasm_std::{coin, Addr, Coin, Decimal, Uint128};

use incentive_manager::ContractError;
use white_whale_std::incentive_manager::{IncentiveAction, IncentiveParams, IncentivesBy};

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
        },
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
        },
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
        },
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

#[test]
fn create_incentives() {
    let lp_denom = "factory/pool/uLP".to_string();

    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_000u128, "uwhale".to_string()),
        coin(1_000_000_000u128, "ulab".to_string()),
        coin(1_000_000_000u128, "uosmo".to_string()),
        coin(1_000_000_000u128, lp_denom.clone()),
    ]);

    let creator = suite.creator();
    let other = suite.senders[1].clone();

    // try all misconfigurations when creating an incentive
    suite
        .instantiate_default()
        .manage_incentive(
            creator.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom.clone(),
                    start_epoch: Some(25),
                    preliminary_end_epoch: None,
                    curve: None,
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Default::default(),
                    },
                    incentive_identifier: None,
                },
            },
            vec![],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::InvalidIncentiveAmount { .. } => {}
                    _ => panic!(
                        "Wrong error type, should return ContractError::InvalidIncentiveAmount"
                    ),
                }
            },
        )
        .manage_incentive(
            other.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom.clone(),
                    start_epoch: Some(25),
                    preliminary_end_epoch: None,
                    curve: None,
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(2_000u128),
                    },
                    incentive_identifier: None,
                },
            },
            vec![coin(2_000, "ulab")],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::IncentiveFeeMissing { .. } => {}
                    _ => {
                        panic!("Wrong error type, should return ContractError::IncentiveFeeMissing")
                    }
                }
            },
        )
        .manage_incentive(
            other.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom.clone(),
                    start_epoch: Some(25),
                    preliminary_end_epoch: None,
                    curve: None,
                    incentive_asset: Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(5_000u128),
                    },
                    incentive_identifier: None,
                },
            },
            vec![coin(8_000, "uwhale")],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::AssetMismatch { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::AssetMismatch"),
                }
            },
        )
        .manage_incentive(
            other.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom.clone(),
                    start_epoch: Some(25),
                    preliminary_end_epoch: None,
                    curve: None,
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(2_000u128),
                    },
                    incentive_identifier: None,
                },
            },
            vec![coin(1_000, "uwhale")],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::AssetMismatch { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::AssetMismatch"),
                }
            },
        )
        .manage_incentive(
            other.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom.clone(),
                    start_epoch: Some(25),
                    preliminary_end_epoch: None,
                    curve: None,
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(2_000u128),
                    },
                    incentive_identifier: None,
                },
            },
            vec![coin(5_000, "ulab"), coin(1_000, "uwhale")],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::AssetMismatch { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::AssetMismatch"),
                }
            },
        )
        .manage_incentive(
            other.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom.clone(),
                    start_epoch: Some(25),
                    preliminary_end_epoch: None,
                    curve: None,
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(5_000u128),
                    },
                    incentive_identifier: None,
                },
            },
            vec![coin(4_000, "ulab"), coin(1_000, "uwhale")],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::AssetMismatch { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::AssetMismatch"),
                }
            },
        )
        .manage_incentive(
            other.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom.clone(),
                    start_epoch: Some(25),
                    preliminary_end_epoch: None,
                    curve: None,
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(4_000u128),
                    },
                    incentive_identifier: None,
                },
            },
            vec![coin(4_000, "ulab"), coin(1_000, "uwhale")],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::IncentiveStartTimeAfterEndTime { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::IncentiveStartTimeAfterEndTime"),
                }
            },
        )
        .manage_incentive(
            other.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom.clone(),
                    start_epoch: Some(20),
                    preliminary_end_epoch: Some(8),
                    curve: None,
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(4_000u128),
                    },
                    incentive_identifier: None,
                },
            },
            vec![coin(4_000, "ulab"), coin(1_000, "uwhale")],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::IncentiveEndsInPast { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::IncentiveEndsInPast"),
                }
            },
        ).manage_incentive(
        other.clone(),
        IncentiveAction::Fill {
            params: IncentiveParams {
                lp_denom: lp_denom.clone(),
                start_epoch: Some(20),
                preliminary_end_epoch: Some(15),
                curve: None,
                incentive_asset: Coin {
                    denom: "ulab".to_string(),
                    amount: Uint128::new(4_000u128),
                },
                incentive_identifier: None,
            },
        },
        vec![coin(4_000, "ulab"), coin(1_000, "uwhale")],
        |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();

            match err {
                ContractError::IncentiveStartTimeAfterEndTime { .. } => {}
                _ => panic!("Wrong error type, should return ContractError::IncentiveStartTimeAfterEndTime"),
            }
        },
    ).manage_incentive(
        other.clone(),
        IncentiveAction::Fill {
            params: IncentiveParams {
                lp_denom: lp_denom.clone(),
                start_epoch: Some(20),
                preliminary_end_epoch: Some(20),
                curve: None,
                incentive_asset: Coin {
                    denom: "ulab".to_string(),
                    amount: Uint128::new(4_000u128),
                },
                incentive_identifier: None,
            },
        },
        vec![coin(4_000, "ulab"), coin(1_000, "uwhale")],
        |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();

            match err {
                ContractError::IncentiveStartTimeAfterEndTime { .. } => {}
                _ => panic!("Wrong error type, should return ContractError::IncentiveStartTimeAfterEndTime"),
            }
        },
    ).manage_incentive(
        other.clone(),
        IncentiveAction::Fill {
            params: IncentiveParams {
                lp_denom: lp_denom.clone(),
                start_epoch: Some(30),
                preliminary_end_epoch: Some(35),
                curve: None,
                incentive_asset: Coin {
                    denom: "ulab".to_string(),
                    amount: Uint128::new(4_000u128),
                },
                incentive_identifier: None,
            },
        },
        vec![coin(4_000, "ulab"), coin(1_000, "uwhale")],
        |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();

            match err {
                ContractError::IncentiveStartTooFar { .. } => {}
                _ => panic!("Wrong error type, should return ContractError::IncentiveStartTooFar"),
            }
        },
    );

    // create an incentive properly
    suite
        .manage_incentive(
            other.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom.clone(),
                    start_epoch: Some(20),
                    preliminary_end_epoch: Some(28),
                    curve: None,
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(4_000u128),
                    },
                    incentive_identifier: Some("incentive_1".to_string()),
                },
            },
            vec![coin(4_000, "ulab"), coin(1_000, "uwhale")],
            |result| {
                result.unwrap();
            },
        )
        .manage_incentive(
            other.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom.clone(),
                    start_epoch: Some(20),
                    preliminary_end_epoch: Some(28),
                    curve: None,
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(10_000u128),
                    },
                    incentive_identifier: None,
                },
            },
            vec![coin(10_000, "ulab"), coin(1_000, "uwhale")],
            |result| {
                result.unwrap();
            },
        )
        .manage_incentive(
            other.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom.clone(),
                    start_epoch: Some(20),
                    preliminary_end_epoch: Some(28),
                    curve: None,
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(4_000u128),
                    },
                    incentive_identifier: None,
                },
            },
            vec![coin(4_000, "ulab"), coin(1_000, "uwhale")],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                // should fail, max incentives per lp_denom was set to 2 in the instantiate_default
                // function
                match err {
                    ContractError::TooManyIncentives { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::TooManyIncentives"),
                }
            },
        )
        .query_incentives(None, None, None, |result| {
            let incentives_response = result.unwrap();
            assert_eq!(incentives_response.incentives.len(), 2);
        })
        .query_incentives(
            Some(IncentivesBy::Identifier("incentive_1".to_string())),
            None,
            None,
            |result| {
                let incentives_response = result.unwrap();
                assert_eq!(incentives_response.incentives.len(), 1);
                assert_eq!(
                    incentives_response.incentives[0].incentive_asset,
                    Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(4_000)
                    }
                );
            },
        )
        .query_incentives(
            Some(IncentivesBy::Identifier("2".to_string())),
            None,
            None,
            |result| {
                let incentives_response = result.unwrap();
                assert_eq!(incentives_response.incentives.len(), 1);
                assert_eq!(
                    incentives_response.incentives[0].incentive_asset,
                    Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(10_000)
                    }
                );
            },
        )
        .query_incentives(
            Some(IncentivesBy::IncentiveAsset("ulab".to_string())),
            None,
            None,
            |result| {
                let incentives_response = result.unwrap();
                assert_eq!(incentives_response.incentives.len(), 2);
            },
        )
        .query_incentives(
            Some(IncentivesBy::LPDenom(lp_denom.clone())),
            None,
            None,
            |result| {
                let incentives_response = result.unwrap();
                assert_eq!(incentives_response.incentives.len(), 2);
            },
        );
}

#[test]
fn expand_incentives() {
    let lp_denom = "factory/pool/uLP".to_string();

    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_000u128, "uwhale".to_string()),
        coin(1_000_000_000u128, "ulab".to_string()),
        coin(1_000_000_000u128, "uosmo".to_string()),
        coin(1_000_000_000u128, lp_denom.clone()),
    ]);

    let creator = suite.creator();
    let other = suite.senders[1].clone();

    suite
        .instantiate_default()
        .manage_incentive(
            other.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom.clone(),
                    start_epoch: Some(20),
                    preliminary_end_epoch: Some(28),
                    curve: None,
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(4_000u128),
                    },
                    incentive_identifier: Some("incentive_1".to_string()),
                },
            },
            vec![coin(4_000, "ulab"), coin(1_000, "uwhale")],
            |result| {
                result.unwrap();
            },
        )
        .manage_incentive(
            creator.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom.clone(),
                    start_epoch: Some(20),
                    preliminary_end_epoch: Some(28),
                    curve: None,
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(8_000u128),
                    },
                    incentive_identifier: Some("incentive_1".to_string()),
                },
            },
            vec![coin(4_000, "ulab")],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::Unauthorized { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::Unauthorized"),
                }
            },
        )
        .manage_incentive(
            other.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom.clone(),
                    start_epoch: Some(20),
                    preliminary_end_epoch: Some(28),
                    curve: None,
                    incentive_asset: Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(8_000u128),
                    },
                    incentive_identifier: Some("incentive_1".to_string()),
                },
            },
            vec![coin(8_000, "uwhale")],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::AssetMismatch { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::AssetMismatch"),
                }
            },
        )
        .manage_incentive(
            other.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom.clone(),
                    start_epoch: Some(20),
                    preliminary_end_epoch: Some(28),
                    curve: None,
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(4_100u128),
                    },
                    incentive_identifier: Some("incentive_1".to_string()),
                },
            },
            vec![coin(4_100, "ulab")],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::InvalidExpansionAmount { .. } => {}
                    _ => panic!(
                        "Wrong error type, should return ContractError::InvalidExpansionAmount"
                    ),
                }
            },
        )
        .query_incentives(
            Some(IncentivesBy::Identifier("incentive_1".to_string())),
            None,
            None,
            |result| {
                let incentives_response = result.unwrap();
                let incentive = incentives_response.incentives[0].clone();
                assert_eq!(
                    incentive.incentive_asset,
                    Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(4_000)
                    }
                );

                assert_eq!(incentive.preliminary_end_epoch, 28);
            },
        )
        .manage_incentive(
            other.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom.clone(),
                    start_epoch: Some(20),
                    preliminary_end_epoch: Some(28),
                    curve: None,
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(5_000u128),
                    },
                    incentive_identifier: Some("incentive_1".to_string()),
                },
            },
            vec![coin(5_000u128, "ulab")],
            |result| {
                result.unwrap();
            },
        )
        .query_incentives(
            Some(IncentivesBy::Identifier("incentive_1".to_string())),
            None,
            None,
            |result| {
                let incentives_response = result.unwrap();
                let incentive = incentives_response.incentives[0].clone();
                assert_eq!(
                    incentive.incentive_asset,
                    Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(9_000)
                    }
                );

                assert_eq!(incentive.preliminary_end_epoch, 38);
            },
        );
}
#[test]
fn close_incentives() {
    let lp_denom = "factory/pool/uLP".to_string();

    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_000u128, "uwhale".to_string()),
        coin(1_000_000_000u128, "ulab".to_string()),
        coin(1_000_000_000u128, "uosmo".to_string()),
        coin(1_000_000_000u128, lp_denom.clone()),
    ]);

    let creator = suite.creator();
    let other = suite.senders[1].clone();
    let another = suite.senders[2].clone();

    suite
        .instantiate_default()
        .manage_incentive(
            other.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom.clone(),
                    start_epoch: Some(20),
                    preliminary_end_epoch: Some(28),
                    curve: None,
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(4_000u128),
                    },
                    incentive_identifier: Some("incentive_1".to_string()),
                },
            },
            vec![coin(4_000, "ulab"), coin(1_000, "uwhale")],
            |result| {
                result.unwrap();
            },
        )
        .manage_incentive(
            other.clone(),
            IncentiveAction::Close {
                incentive_identifier: "incentive_1".to_string(),
            },
            vec![coin(1_000, "uwhale")],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::PaymentError { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::PaymentError"),
                }
            },
        )
        .manage_incentive(
            other.clone(),
            IncentiveAction::Close {
                incentive_identifier: "incentive_2".to_string(),
            },
            vec![],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::NonExistentIncentive { .. } => {}
                    _ => panic!(
                        "Wrong error type, should return ContractError::NonExistentIncentive"
                    ),
                }
            },
        )
        .manage_incentive(
            another.clone(),
            IncentiveAction::Close {
                incentive_identifier: "incentive_1".to_string(),
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
        .query_balance("ulab".to_string(), other.clone(), |balance| {
            assert_eq!(balance, Uint128::new(99_999_6000));
        })
        .manage_incentive(
            other.clone(),
            IncentiveAction::Close {
                incentive_identifier: "incentive_1".to_string(),
            },
            vec![],
            |result| {
                result.unwrap();
            },
        )
        .query_balance("ulab".to_string(), other.clone(), |balance| {
            assert_eq!(balance, Uint128::new(100_000_0000));
        });

    suite
        .instantiate_default()
        .manage_incentive(
            other.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom.clone(),
                    start_epoch: Some(20),
                    preliminary_end_epoch: Some(28),
                    curve: None,
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(4_000u128),
                    },
                    incentive_identifier: Some("incentive_1".to_string()),
                },
            },
            vec![coin(4_000, "ulab"), coin(1_000, "uwhale")],
            |result| {
                result.unwrap();
            },
        )
        .query_balance("ulab".to_string(), other.clone(), |balance| {
            assert_eq!(balance, Uint128::new(99_999_6000));
        })
        // the owner of the contract can also close incentives
        .manage_incentive(
            creator.clone(),
            IncentiveAction::Close {
                incentive_identifier: "incentive_1".to_string(),
            },
            vec![],
            |result| {
                result.unwrap();
            },
        )
        .query_balance("ulab".to_string(), other.clone(), |balance| {
            assert_eq!(balance, Uint128::new(100_000_0000));
        });
}
