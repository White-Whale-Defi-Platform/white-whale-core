extern crate core;

use cosmwasm_std::{coin, Addr, Coin, Decimal, Uint128};

use incentive_manager::ContractError;
use white_whale_std::incentive_manager::{
    Config, Curve, Incentive, IncentiveAction, IncentiveParams, IncentivesBy, LpWeightResponse,
    Position, PositionAction, RewardsResponse,
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
                ContractError::InvalidUnlockingRange { .. } => {}
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
                    ContractError::IncentiveStartTooFar { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::IncentiveStartTooFar"),
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
                start_epoch: Some(3),
                preliminary_end_epoch: Some(5),
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
    )

        .manage_incentive(
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
                        amount: Uint128::new(4_000),
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
                        amount: Uint128::new(10_000),
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
                        amount: Uint128::new(4_000),
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
                        amount: Uint128::new(9_000),
                    }
                );

                assert_eq!(incentive.preliminary_end_epoch, 38);
            },
        );
}

#[test]
#[allow(clippy::inconsistent_digit_grouping)]
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
            assert_eq!(balance, Uint128::new(999_996_000));
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
            assert_eq!(balance, Uint128::new(1000_000_000));
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
            assert_eq!(balance, Uint128::new(999_996_000));
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
            assert_eq!(balance, Uint128::new(1000_000_000));
        });
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
fn test_epoch_change_hook() {}

#[test]
pub fn update_config() {
    let lp_denom = "factory/pool/uLP".to_string();

    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_000u128, "uwhale".to_string()),
        coin(1_000_000_000u128, "ulab".to_string()),
        coin(1_000_000_000u128, "uosmo".to_string()),
        coin(1_000_000_000u128, lp_denom.clone()),
    ]);

    let creator = suite.creator();
    let other = suite.senders[1].clone();

    suite.instantiate_default();

    let whale_lair = suite.whale_lair_addr.clone();
    let epoch_manager = suite.epoch_manager_addr.clone();

    let expected_config = Config {
        whale_lair_addr: whale_lair,
        epoch_manager_addr: epoch_manager,
        create_incentive_fee: Coin {
            denom: "uwhale".to_string(),
            amount: Uint128::new(1_000u128),
        },
        max_concurrent_incentives: 2u32,
        max_incentive_epoch_buffer: 14u32,
        min_unlocking_duration: 86_400u64,
        max_unlocking_duration: 31_536_000u64,
        emergency_unlock_penalty: Decimal::percent(10),
    };

    suite.query_config(|result| {
        let config = result.unwrap();
        assert_eq!(config, expected_config);
    })
        .update_config(
            other.clone(),
            Some(MOCK_CONTRACT_ADDR.to_string()),
            Some(MOCK_CONTRACT_ADDR.to_string()),
            Some(Coin {
                denom: "uwhale".to_string(),
                amount: Uint128::new(2_000u128),
            }),
            Some(3u32),
            Some(15u32),
            Some(172_800u64),
            Some(864_000u64),
            Some(Decimal::percent(50)),
            vec![coin(1_000, "uwhale")],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::PaymentError { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::PaymentError"),
                }
            },
        ).update_config(
        other.clone(),
        Some(MOCK_CONTRACT_ADDR.to_string()),
        Some(MOCK_CONTRACT_ADDR.to_string()),
        Some(Coin {
            denom: "uwhale".to_string(),
            amount: Uint128::new(2_000u128),
        }),
        Some(0u32),
        Some(15u32),
        Some(172_800u64),
        Some(864_000u64),
        Some(Decimal::percent(50)),
        vec![],
        |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();
            match err {
                ContractError::OwnershipError { .. } => {}
                _ => panic!("Wrong error type, should return ContractError::OwnershipError"),
            }
        },
    ).update_config(
        creator.clone(),
        Some(MOCK_CONTRACT_ADDR.to_string()),
        Some(MOCK_CONTRACT_ADDR.to_string()),
        Some(Coin {
            denom: "uwhale".to_string(),
            amount: Uint128::new(2_000u128),
        }),
        Some(0u32),
        Some(15u32),
        Some(172_800u64),
        Some(864_000u64),
        Some(Decimal::percent(50)),
        vec![],
        |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();
            match err {
                ContractError::UnspecifiedConcurrentIncentives { .. } => {}
                _ => panic!("Wrong error type, should return ContractError::UnspecifiedConcurrentIncentives"),
            }
        },
    ).update_config(
        creator.clone(),
        Some(MOCK_CONTRACT_ADDR.to_string()),
        Some(MOCK_CONTRACT_ADDR.to_string()),
        Some(Coin {
            denom: "uwhale".to_string(),
            amount: Uint128::new(2_000u128),
        }),
        Some(5u32),
        Some(15u32),
        Some(80_800u64),
        Some(80_000u64),
        Some(Decimal::percent(50)),
        vec![],
        |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();
            match err {
                ContractError::InvalidUnlockingRange { .. } => {}
                _ => panic!("Wrong error type, should return ContractError::InvalidUnbondingRange"),
            }
        },
    ).update_config(
        creator.clone(),
        Some(MOCK_CONTRACT_ADDR.to_string()),
        Some(MOCK_CONTRACT_ADDR.to_string()),
        Some(Coin {
            denom: "uwhale".to_string(),
            amount: Uint128::new(2_000u128),
        }),
        Some(5u32),
        Some(15u32),
        Some(300_000u64),
        Some(200_000u64),
        Some(Decimal::percent(50)),
        vec![],
        |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();
            match err {
                ContractError::InvalidUnlockingRange { .. } => {}
                _ => panic!("Wrong error type, should return ContractError::InvalidUnbondingRange"),
            }
        },
    ).update_config(
        creator.clone(),
        Some(MOCK_CONTRACT_ADDR.to_string()),
        Some(MOCK_CONTRACT_ADDR.to_string()),
        Some(Coin {
            denom: "uwhale".to_string(),
            amount: Uint128::new(2_000u128),
        }),
        Some(5u32),
        Some(15u32),
        Some(100_000u64),
        Some(200_000u64),
        Some(Decimal::percent(105)),
        vec![],
        |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();
            match err {
                ContractError::InvalidEmergencyUnlockPenalty { .. } => {}
                _ => panic!("Wrong error type, should return ContractError::InvalidEmergencyUnlockPenalty"),
            }
        },
    ).update_config(
        creator.clone(),
        Some(MOCK_CONTRACT_ADDR.to_string()),
        Some(MOCK_CONTRACT_ADDR.to_string()),
        Some(Coin {
            denom: "uwhale".to_string(),
            amount: Uint128::new(2_000u128),
        }),
        Some(5u32),
        Some(15u32),
        Some(100_000u64),
        Some(200_000u64),
        Some(Decimal::percent(20)),
        vec![],
        |result| {
            result.unwrap();
        },
    );

    let expected_config = Config {
        whale_lair_addr: Addr::unchecked(MOCK_CONTRACT_ADDR),
        epoch_manager_addr: Addr::unchecked(MOCK_CONTRACT_ADDR),
        create_incentive_fee: Coin {
            denom: "uwhale".to_string(),
            amount: Uint128::new(2_000u128),
        },
        max_concurrent_incentives: 5u32,
        max_incentive_epoch_buffer: 15u32,
        min_unlocking_duration: 100_000u64,
        max_unlocking_duration: 200_000u64,
        emergency_unlock_penalty: Decimal::percent(20),
    };

    suite.query_config(|result| {
        let config = result.unwrap();
        assert_eq!(config, expected_config);
    });
}

#[test]
#[allow(clippy::inconsistent_digit_grouping)]
pub fn test_manage_position() {
    let lp_denom = "factory/pool/uLP".to_string();

    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_000u128, "uwhale"),
        coin(1_000_000_000u128, "ulab"),
        coin(1_000_000_000u128, "uosmo"),
        coin(1_000_000_000u128, lp_denom.clone()),
        coin(1_000_000_000u128, "invalid_lp"),
    ]);

    let creator = suite.creator();
    let other = suite.senders[1].clone();
    let another = suite.senders[2].clone();

    suite.instantiate_default();

    let incentive_manager = suite.incentive_manager_addr.clone();
    let whale_lair = suite.whale_lair_addr.clone();

    suite
        .add_hook(creator.clone(), incentive_manager, vec![], |result| {
            result.unwrap();
        })
        .manage_incentive(
            creator.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom.clone(),
                    start_epoch: Some(12),
                    preliminary_end_epoch: Some(16),
                    curve: None,
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(8_000u128),
                    },
                    incentive_identifier: None,
                },
            },
            vec![coin(8_000, "ulab"), coin(1_000, "uwhale")],
            |result| {
                result.unwrap();
            },
        )
        .query_lp_weight(&lp_denom, 10, |result| {
            let err = result.unwrap_err().to_string();

            assert_eq!(
                err,
                "Generic error: Querier contract error: There's no snapshot of the LP \
           weight in the contract for the epoch 10"
            );
        })
        .manage_position(
            creator.clone(),
            PositionAction::Fill {
                identifier: Some("creator_position".to_string()),
                unlocking_duration: 80_400,
                receiver: None,
            },
            vec![coin(1_000, lp_denom.clone())],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::InvalidUnlockingDuration { .. } => {}
                    _ => panic!(
                        "Wrong error type, should return ContractError::InvalidUnlockingDuration"
                    ),
                }
            },
        )
        .manage_position(
            creator.clone(),
            PositionAction::Fill {
                identifier: Some("creator_position".to_string()),
                unlocking_duration: 32_536_000,
                receiver: None,
            },
            vec![coin(1_000, lp_denom.clone())],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::InvalidUnlockingDuration { .. } => {}
                    _ => panic!(
                        "Wrong error type, should return ContractError::InvalidUnlockingDuration"
                    ),
                }
            },
        )
        .manage_position(
            creator.clone(),
            PositionAction::Fill {
                identifier: Some("creator_position".to_string()),
                unlocking_duration: 32_536_000,
                receiver: None,
            },
            vec![],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::PaymentError { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::PaymentError"),
                }
            },
        )
        .manage_position(
            creator.clone(),
            PositionAction::Fill {
                identifier: Some("creator_position".to_string()),
                unlocking_duration: 86_400,
                receiver: None,
            },
            vec![coin(1_000, lp_denom.clone())],
            |result| {
                result.unwrap();
            },
        )
        .query_lp_weight(&lp_denom, 11, |result| {
            let lp_weight = result.unwrap();
            assert_eq!(
                lp_weight,
                LpWeightResponse {
                    lp_weight: Uint128::new(1_000),
                    epoch_id: 11,
                }
            );
        })
        .manage_position(
            creator.clone(),
            PositionAction::Fill {
                identifier: Some("creator_position".to_string()),
                unlocking_duration: 86_400,
                receiver: None,
            },
            vec![coin(1_000, "invalid_lp".to_string())],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::AssetMismatch { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::AssetMismatch"),
                }
            },
        )
        .query_positions(creator.clone(), Some(true), |result| {
            let positions = result.unwrap();
            assert_eq!(positions.positions.len(), 1);
            assert_eq!(
                positions.positions[0],
                Position {
                    identifier: "creator_position".to_string(),
                    lp_asset: Coin {
                        denom: "factory/pool/uLP".to_string(),
                        amount: Uint128::new(1_000),
                    },
                    unlocking_duration: 86400,
                    open: true,
                    expiring_at: None,
                    receiver: Addr::unchecked("migaloo1h3s5np57a8cxaca3rdjlgu8jzmr2d2zz55s5y3"),
                }
            );
        })
        .manage_position(
            creator.clone(),
            PositionAction::Fill {
                identifier: Some("creator_position".to_string()),
                unlocking_duration: 86_400,
                receiver: None,
            },
            vec![coin(5_000, lp_denom.clone())],
            |result| {
                result.unwrap();
            },
        )
        .manage_position(
            creator.clone(),
            PositionAction::Withdraw {
                identifier: "creator_position".to_string(),
                emergency_unlock: None,
            },
            vec![],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                // the position is not closed or hasn't expired yet
                match err {
                    ContractError::Unauthorized { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::Unauthorized"),
                }
            },
        )
        .query_lp_weight(&lp_denom, 11, |result| {
            let lp_weight = result.unwrap();
            assert_eq!(
                lp_weight,
                LpWeightResponse {
                    lp_weight: Uint128::new(6_000),
                    epoch_id: 11,
                }
            );
        })
        .query_positions(creator.clone(), Some(true), |result| {
            let positions = result.unwrap();
            assert_eq!(positions.positions.len(), 1);
            assert_eq!(
                positions.positions[0],
                Position {
                    identifier: "creator_position".to_string(),
                    lp_asset: Coin {
                        denom: "factory/pool/uLP".to_string(),
                        amount: Uint128::new(6_000),
                    },
                    unlocking_duration: 86400,
                    open: true,
                    expiring_at: None,
                    receiver: Addr::unchecked("migaloo1h3s5np57a8cxaca3rdjlgu8jzmr2d2zz55s5y3"),
                }
            );
        })
        .query_lp_weight(&lp_denom, 11, |result| {
            let lp_weight = result.unwrap();
            assert_eq!(
                lp_weight,
                LpWeightResponse {
                    lp_weight: Uint128::new(6_000),
                    epoch_id: 11,
                }
            );
        })
        .add_one_day()
        .create_epoch(creator.clone(), |result| {
            result.unwrap();
        })
        .query_current_epoch(|result| {
            let epoch_response = result.unwrap();
            assert_eq!(epoch_response.epoch.id, 11);
        });

    // make sure snapshots are working correctly
    suite
        .query_lp_weight(&lp_denom, 15, |result| {
            let err = result.unwrap_err().to_string();

            assert_eq!(
                err,
                "Generic error: Querier contract error: There's no snapshot of the LP weight in the \
            contract for the epoch 15"
            );
        })
        .add_one_day()
        .create_epoch(creator.clone(), |result| {
            result.unwrap();
        })
        .query_current_epoch(|result| {
            let epoch_response = result.unwrap();
            assert_eq!(epoch_response.epoch.id, 12);
        })
        .query_lp_weight(&lp_denom, 12, |result| {
            let lp_weight = result.unwrap();
            assert_eq!(
                lp_weight,
                LpWeightResponse {
                    lp_weight: Uint128::new(6_000), //snapshot taken from the previous epoch
                    epoch_id: 12,
                }
            );
        })
        .manage_position(
            creator.clone(),
            PositionAction::Fill {
                //refill position
                identifier: Some("creator_position".to_string()),
                unlocking_duration: 86_400,
                receiver: None,
            },
            vec![coin(1_000, lp_denom.clone())],
            |result| {
                result.unwrap();
            },
        )
        .query_lp_weight(&lp_denom, 12, |result| {
            let lp_weight = result.unwrap();
            assert_eq!(
                lp_weight,
                LpWeightResponse {
                    // should be the same for epoch 12, as the weight for new positions is added
                    // to the next epoch
                    lp_weight: Uint128::new(6_000),
                    epoch_id: 12,
                }
            );
        });

    suite.query_current_epoch(|result| {
        let epoch_response = result.unwrap();
        assert_eq!(epoch_response.epoch.id, 12);
    });

    suite
        .manage_position(
            creator.clone(),
            PositionAction::Close {
                identifier: "creator_position".to_string(),
                lp_asset: Some(Coin {
                    denom: lp_denom.clone(),
                    amount: Uint128::new(4_000),
                }),
            },
            vec![coin(4_000, lp_denom.clone())],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::PaymentError { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::PaymentError"),
                }
            },
        )
        .manage_position(
            creator.clone(),
            PositionAction::Close {
                // remove 4_000 from the 7_000 position
                identifier: "creator_position".to_string(),
                lp_asset: Some(Coin {
                    denom: lp_denom.clone(),
                    amount: Uint128::new(4_000),
                }),
            },
            vec![],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::PendingRewards { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::PendingRewards"),
                }
            },
        )
        .claim(
            creator.clone(),
            vec![coin(4_000, lp_denom.clone())],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::PaymentError { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::PaymentError"),
                }
            },
        )
        .claim(other.clone(), vec![], |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();
            match err {
                ContractError::NoOpenPositions { .. } => {}
                _ => panic!("Wrong error type, should return ContractError::NoOpenPositions"),
            }
        })
        .query_balance("ulab".to_string(), creator.clone(), |balance| {
            assert_eq!(balance, Uint128::new(999_992_000));
        })
        .claim(creator.clone(), vec![], |result| {
            result.unwrap();
        })
        .query_balance("ulab".to_string(), creator.clone(), |balance| {
            assert_eq!(balance, Uint128::new(999_994_000));
        })
        .query_incentives(None, None, None, |result| {
            let incentives_response = result.unwrap();
            assert_eq!(incentives_response.incentives.len(), 1);
            assert_eq!(
                incentives_response.incentives[0].claimed_amount,
                Uint128::new(2_000),
            );
        })
        .manage_position(
            creator.clone(),
            PositionAction::Close {
                identifier: "non_existent__position".to_string(),
                lp_asset: Some(Coin {
                    denom: lp_denom.clone(),
                    amount: Uint128::new(4_000),
                }),
            },
            vec![],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::NoPositionFound { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::NoPositionFound"),
                }
            },
        )
        .manage_position(
            other.clone(),
            PositionAction::Close {
                identifier: "creator_position".to_string(),
                lp_asset: Some(Coin {
                    denom: lp_denom.clone(),
                    amount: Uint128::new(4_000),
                }),
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
        .manage_position(
            creator.clone(),
            PositionAction::Close {
                identifier: "creator_position".to_string(),
                lp_asset: Some(Coin {
                    denom: "invalid_lp".to_string(),
                    amount: Uint128::new(4_000),
                }),
            },
            vec![],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::AssetMismatch { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::AssetMismatch"),
                }
            },
        )
        .manage_position(
            creator.clone(), // someone tries to close the creator's position
            PositionAction::Close {
                identifier: "creator_position".to_string(),
                lp_asset: Some(Coin {
                    denom: lp_denom.to_string(),
                    amount: Uint128::new(10_000),
                }),
            },
            vec![],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::AssetMismatch { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::AssetMismatch"),
                }
            },
        )
        .manage_position(
            creator.clone(),
            PositionAction::Close {
                // remove 5_000 from the 7_000 position
                identifier: "creator_position".to_string(),
                lp_asset: Some(Coin {
                    denom: lp_denom.clone(),
                    amount: Uint128::new(5_000),
                }),
            },
            vec![],
            |result| {
                result.unwrap();
            },
        )
        .manage_position(
            creator.clone(),
            PositionAction::Withdraw {
                identifier: "2".to_string(),
                emergency_unlock: None,
            },
            vec![],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::PositionNotExpired { .. } => {}
                    _ => {
                        panic!("Wrong error type, should return ContractError::PositionNotExpired")
                    }
                }
            },
        )
        .query_lp_weight(&lp_denom, 12, |result| {
            let lp_weight = result.unwrap();
            assert_eq!(
                lp_weight,
                LpWeightResponse {
                    // should be the same for epoch 12, as the weight for new positions is added
                    // to the next epoch
                    lp_weight: Uint128::new(6_000),
                    epoch_id: 12,
                }
            );
        })
        .query_lp_weight(&lp_denom, 13, |result| {
            let lp_weight = result.unwrap();
            assert_eq!(
                lp_weight,
                LpWeightResponse {
                    // should be the same for epoch 12, as the weight for new positions is added
                    // to the next epoch
                    lp_weight: Uint128::new(5_000),
                    epoch_id: 13,
                }
            );
        })
        // create a few epochs without any changes in the weight
        .add_one_day()
        .create_epoch(creator.clone(), |result| {
            result.unwrap();
        })
        //after a day the closed position should be able to be withdrawn
        .manage_position(
            other.clone(),
            PositionAction::Withdraw {
                identifier: "creator_position".to_string(),
                emergency_unlock: None,
            },
            vec![coin(5_000, lp_denom.clone())],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::PaymentError { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::PaymentError"),
                }
            },
        )
        .manage_position(
            creator.clone(),
            PositionAction::Withdraw {
                identifier: "non_existent_position".to_string(),
                emergency_unlock: None,
            },
            vec![],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::NoPositionFound { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::NoPositionFound"),
                }
            },
        )
        .manage_position(
            other.clone(),
            PositionAction::Withdraw {
                identifier: "2".to_string(),
                emergency_unlock: None,
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
        .add_one_day()
        .create_epoch(creator.clone(), |result| {
            result.unwrap();
        })
        .add_one_day()
        .create_epoch(creator.clone(), |result| {
            result.unwrap();
        })
        .query_lp_weight(&lp_denom, 14, |result| {
            let lp_weight = result.unwrap();
            assert_eq!(
                lp_weight,
                LpWeightResponse {
                    // should be the same for epoch 13, as nobody changed their positions
                    lp_weight: Uint128::new(5_000),
                    epoch_id: 14,
                }
            );
        })
        .query_lp_weight(&lp_denom, 15, |result| {
            let lp_weight = result.unwrap();
            assert_eq!(
                lp_weight,
                LpWeightResponse {
                    // should be the same for epoch 13, as nobody changed their positions
                    lp_weight: Uint128::new(5_000),
                    epoch_id: 15,
                }
            );
        })
        .query_current_epoch(|result| {
            let epoch_response = result.unwrap();
            assert_eq!(epoch_response.epoch.id, 15);
        })
        .add_one_day()
        .create_epoch(creator.clone(), |result| {
            result.unwrap();
        })
        .query_rewards(creator.clone(), |result| {
            let rewards_response = result.unwrap();
            match rewards_response {
                RewardsResponse::RewardsResponse { rewards } => {
                    assert_eq!(rewards.len(), 1);
                    assert_eq!(
                        rewards[0],
                        Coin {
                            denom: "ulab".to_string(),
                            amount: Uint128::new(6_000),
                        }
                    );
                }
                RewardsResponse::ClaimRewards { .. } => {
                    panic!("shouldn't return this but RewardsResponse")
                }
            }
        })
        .query_incentives(None, None, None, |result| {
            let incentives_response = result.unwrap();
            assert_eq!(
                incentives_response.incentives[0].claimed_amount,
                Uint128::new(2_000)
            );
        })
        .manage_position(
            creator.clone(),
            PositionAction::Withdraw {
                identifier: "2".to_string(),
                emergency_unlock: None,
            },
            vec![],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::PendingRewards { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::PendingRewards"),
                }
            },
        )
        .claim(creator.clone(), vec![], |result| {
            result.unwrap();
        })
        .query_balance("ulab".to_string(), creator.clone(), |balance| {
            assert_eq!(balance, Uint128::new(1000_000_000));
        })
        .query_incentives(None, None, None, |result| {
            let incentives_response = result.unwrap();
            assert_eq!(
                incentives_response.incentives[0].incentive_asset.amount,
                incentives_response.incentives[0].claimed_amount
            );
            assert!(incentives_response.incentives[0].is_expired(15));
        })
        .query_rewards(creator.clone(), |result| {
            let rewards_response = result.unwrap();
            match rewards_response {
                RewardsResponse::RewardsResponse { rewards } => {
                    assert!(rewards.is_empty());
                }
                RewardsResponse::ClaimRewards { .. } => {
                    panic!("shouldn't return this but RewardsResponse")
                }
            }
        })
        .claim(creator.clone(), vec![], |result| {
            result.unwrap();
        })
        .query_balance("ulab".to_string(), creator.clone(), |balance| {
            assert_eq!(balance, Uint128::new(1000_000_000));
        })
        .manage_position(
            creator.clone(),
            PositionAction::Withdraw {
                identifier: "2".to_string(),
                emergency_unlock: None,
            },
            vec![],
            |result| {
                result.unwrap();
            },
        )
        .query_positions(other.clone(), Some(false), |result| {
            let positions = result.unwrap();
            assert!(positions.positions.is_empty());
        })
        .manage_position(
            creator.clone(),
            PositionAction::Fill {
                identifier: None,
                unlocking_duration: 86_400,
                receiver: Some(another.clone().to_string()),
            },
            vec![coin(5_000, lp_denom.clone())],
            |result| {
                result.unwrap();
            },
        )
        .query_positions(another.clone(), Some(true), |result| {
            let positions = result.unwrap();
            assert_eq!(positions.positions.len(), 1);
            assert_eq!(
                positions.positions[0],
                Position {
                    identifier: "3".to_string(),
                    lp_asset: Coin {
                        denom: "factory/pool/uLP".to_string(),
                        amount: Uint128::new(5_000),
                    },
                    unlocking_duration: 86400,
                    open: true,
                    expiring_at: None,
                    receiver: another.clone(),
                }
            );
        })
        .manage_position(
            creator.clone(),
            PositionAction::Close {
                identifier: "3".to_string(),
                lp_asset: None,
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
        .manage_position(
            another.clone(),
            PositionAction::Close {
                identifier: "3".to_string(),
                lp_asset: None, //close in full
            },
            vec![],
            |result| {
                result.unwrap();
            },
        )
        .query_positions(another.clone(), Some(true), |result| {
            let positions = result.unwrap();
            assert!(positions.positions.is_empty());
        })
        .query_positions(another.clone(), Some(false), |result| {
            let positions = result.unwrap();
            assert_eq!(positions.positions.len(), 1);
            assert_eq!(
                positions.positions[0],
                Position {
                    identifier: "3".to_string(),
                    lp_asset: Coin {
                        denom: "factory/pool/uLP".to_string(),
                        amount: Uint128::new(5_000),
                    },
                    unlocking_duration: 86400,
                    open: false,
                    expiring_at: Some(1712847600),
                    receiver: another.clone(),
                }
            );
        });

    suite
        .add_one_epoch()
        .add_one_epoch()
        .query_current_epoch(|result| {
            let epoch_response = result.unwrap();
            assert_eq!(epoch_response.epoch.id, 18);
        });

    // try emergency exit a position that is closed
    suite
        .manage_position(
            another.clone(),
            PositionAction::Fill {
                identifier: Some("special_position".to_string()),
                unlocking_duration: 100_000,
                receiver: None,
            },
            vec![coin(5_000, lp_denom.clone())],
            |result| {
                result.unwrap();
            },
        )
        .query_lp_weight(&lp_denom, 18, |result| {
            let lp_weight = result.unwrap();
            assert_eq!(
                lp_weight,
                LpWeightResponse {
                    lp_weight: Uint128::new(5_000),
                    epoch_id: 18,
                }
            );
        })
        .query_lp_weight(&lp_denom, 19, |result| {
            let lp_weight = result.unwrap();
            assert_eq!(
                lp_weight,
                LpWeightResponse {
                    lp_weight: Uint128::new(10_002),
                    epoch_id: 19,
                }
            );
        });

    suite.add_one_epoch().query_current_epoch(|result| {
        let epoch_response = result.unwrap();
        assert_eq!(epoch_response.epoch.id, 19);
    });

    // close the position
    suite
        .manage_position(
            another.clone(),
            PositionAction::Close {
                identifier: "special_position".to_string(),
                lp_asset: None,
            },
            vec![],
            |result| {
                result.unwrap();
            },
        )
        .query_lp_weight(&lp_denom, 20, |result| {
            let lp_weight = result.unwrap();
            assert_eq!(
                lp_weight,
                LpWeightResponse {
                    // the weight went back to what it was before the position was opened
                    lp_weight: Uint128::new(5_000),
                    epoch_id: 20,
                }
            );
        });

    // emergency exit
    suite
        .query_balance(
            lp_denom.clone().to_string(),
            whale_lair.clone(),
            |balance| {
                assert_eq!(balance, Uint128::zero());
            },
        )
        .manage_position(
            another.clone(),
            PositionAction::Close {
                identifier: "special_position".to_string(),
                lp_asset: None,
            },
            vec![],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::PositionAlreadyClosed { .. } => {}
                    _ => panic!(
                        "Wrong error type, should return ContractError::PositionAlreadyClosed"
                    ),
                }
            },
        )
        .manage_position(
            another.clone(),
            PositionAction::Withdraw {
                identifier: "special_position".to_string(),
                emergency_unlock: Some(true),
            },
            vec![],
            |result| {
                result.unwrap();
            },
        )
        .query_balance(
            lp_denom.clone().to_string(),
            whale_lair.clone(),
            |balance| {
                assert_eq!(balance, Uint128::new(500));
            },
        );
}

#[test]
fn claim_expired_incentive_returns_nothing() {
    let lp_denom = "factory/pool/uLP".to_string();

    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_000u128, "uwhale"),
        coin(1_000_000_000u128, "ulab"),
        coin(1_000_000_000u128, "uosmo"),
        coin(1_000_000_000u128, lp_denom.clone()),
        coin(1_000_000_000u128, "invalid_lp"),
    ]);

    let creator = suite.creator();
    let other = suite.senders[1].clone();

    suite.instantiate_default();

    let incentive_manager = suite.incentive_manager_addr.clone();

    suite
        .add_hook(creator.clone(), incentive_manager, vec![], |result| {
            result.unwrap();
        })
        .manage_incentive(
            creator.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom.clone(),
                    start_epoch: Some(12),
                    preliminary_end_epoch: Some(16),
                    curve: None,
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(8_000u128),
                    },
                    incentive_identifier: None,
                },
            },
            vec![coin(8_000, "ulab"), coin(1_000, "uwhale")],
            |result| {
                result.unwrap();
            },
        )
        .manage_position(
            other.clone(),
            PositionAction::Fill {
                identifier: Some("creator_position".to_string()),
                unlocking_duration: 86_400,
                receiver: None,
            },
            vec![coin(5_000, lp_denom.clone())],
            |result| {
                result.unwrap();
            },
        )
        .query_lp_weight(&lp_denom, 11, |result| {
            let lp_weight = result.unwrap();
            assert_eq!(
                lp_weight,
                LpWeightResponse {
                    lp_weight: Uint128::new(5_000),
                    epoch_id: 11,
                }
            );
        })
        .query_positions(other.clone(), Some(true), |result| {
            let positions = result.unwrap();
            assert_eq!(positions.positions.len(), 1);
            assert_eq!(
                positions.positions[0],
                Position {
                    identifier: "creator_position".to_string(),
                    lp_asset: Coin {
                        denom: "factory/pool/uLP".to_string(),
                        amount: Uint128::new(5_000),
                    },
                    unlocking_duration: 86400,
                    open: true,
                    expiring_at: None,
                    receiver: Addr::unchecked("migaloo193lk767456jhkzddnz7kf5jvuzfn67gyfvhc40"),
                }
            );
        });

    // create a couple of epochs to make the incentive active

    suite
        .add_one_day()
        .create_epoch(creator.clone(), |result| {
            result.unwrap();
        })
        .add_one_day()
        .create_epoch(creator.clone(), |result| {
            result.unwrap();
        })
        .add_one_day()
        .create_epoch(creator.clone(), |result| {
            result.unwrap();
        })
        .add_one_day()
        .create_epoch(creator.clone(), |result| {
            result.unwrap();
        })
        .query_current_epoch(|result| {
            let epoch_response = result.unwrap();
            assert_eq!(epoch_response.epoch.id, 14);
        })
        .query_balance("ulab".to_string(), other.clone(), |balance| {
            assert_eq!(balance, Uint128::new(1_000_000_000u128));
        })
        .claim(other.clone(), vec![], |result| {
            result.unwrap();
        })
        .query_balance("ulab".to_string(), other.clone(), |balance| {
            assert_eq!(balance, Uint128::new(1_000_006_000u128));
        });

    // create a bunch of epochs to make the incentive expire
    for _ in 0..15 {
        suite.add_one_day().create_epoch(creator.clone(), |result| {
            result.unwrap();
        });
    }

    // there shouldn't be anything to claim as the incentive has expired, even though it still has some funds
    suite
        .query_rewards(creator.clone(), |result| {
            let rewards_response = result.unwrap();
            match rewards_response {
                RewardsResponse::RewardsResponse { rewards } => {
                    assert!(rewards.is_empty());
                }
                RewardsResponse::ClaimRewards { .. } => {
                    panic!("shouldn't return this but RewardsResponse")
                }
            }
        })
        .claim(other.clone(), vec![], |result| {
            result.unwrap();
        })
        .query_balance("ulab".to_string(), other.clone(), |balance| {
            // the balance hasn't changed
            assert_eq!(balance, Uint128::new(1_000_006_000u128));
        });
}

#[test]
fn test_close_expired_incentives() {
    let lp_denom = "factory/pool/uLP".to_string();

    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_000u128, "uwhale"),
        coin(1_000_000_000u128, "ulab"),
        coin(1_000_000_000u128, "uosmo"),
        coin(1_000_000_000u128, lp_denom.clone()),
        coin(1_000_000_000u128, "invalid_lp"),
    ]);

    let creator = suite.creator();
    let other = suite.senders[1].clone();

    suite.instantiate_default();

    let incentive_manager = suite.incentive_manager_addr.clone();

    suite
        .add_hook(creator.clone(), incentive_manager, vec![], |result| {
            result.unwrap();
        })
        .manage_incentive(
            creator.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom.clone(),
                    start_epoch: Some(12),
                    preliminary_end_epoch: Some(16),
                    curve: None,
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(8_000u128),
                    },
                    incentive_identifier: None,
                },
            },
            vec![coin(8_000, "ulab"), coin(1_000, "uwhale")],
            |result| {
                result.unwrap();
            },
        );

    // create a bunch of epochs to make the incentive expire
    for _ in 0..20 {
        suite.add_one_day().create_epoch(creator.clone(), |result| {
            result.unwrap();
        });
    }

    let mut current_id = 0;

    // try opening another incentive for the same lp denom, the expired incentive should get closed
    suite
        .query_current_epoch(|result| {
            let epoch_response = result.unwrap();
            current_id = epoch_response.epoch.id;
        })
        .query_incentives(None, None, None, |result| {
            let incentives_response = result.unwrap();
            assert_eq!(incentives_response.incentives.len(), 1);
            assert!(incentives_response.incentives[0].is_expired(current_id));
        })
        .manage_incentive(
            other.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom.clone(),
                    start_epoch: None,
                    preliminary_end_epoch: None,
                    curve: None,
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(10_000u128),
                    },
                    incentive_identifier: Some("new_incentive".to_string()),
                },
            },
            vec![coin(10_000, "ulab"), coin(1_000, "uwhale")],
            |result| {
                result.unwrap();
            },
        )
        .query_incentives(None, None, None, |result| {
            let incentives_response = result.unwrap();
            assert_eq!(incentives_response.incentives.len(), 1);
            assert_eq!(
                incentives_response.incentives[0],
                Incentive {
                    identifier: "new_incentive".to_string(),
                    owner: other.clone(),
                    lp_denom: lp_denom.clone(),
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(10_000u128),
                    },
                    claimed_amount: Uint128::zero(),
                    emission_rate: Uint128::new(714),
                    curve: Curve::Linear,
                    start_epoch: 30u64,
                    preliminary_end_epoch: 44u64,
                    last_epoch_claimed: 29u64,
                }
            );
        });
}

#[test]
fn on_epoch_changed_unauthorized() {
    let mut suite = TestingSuite::default_with_balances(vec![]);
    let creator = suite.creator();

    suite
        .instantiate_default()
        .on_epoch_changed(creator, vec![], |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();
            match err {
                ContractError::Unauthorized { .. } => {}
                _ => panic!("Wrong error type, should return ContractError::Unauthorized"),
            }
        });
}

#[test]
fn expand_expired_incentive() {
    let lp_denom = "factory/pool/uLP".to_string();

    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_000u128, "uwhale".to_string()),
        coin(1_000_000_000u128, "ulab".to_string()),
        coin(1_000_000_000u128, "uosmo".to_string()),
        coin(1_000_000_000u128, lp_denom.clone()),
    ]);

    let creator = suite.creator();
    let other = suite.senders[1].clone();

    suite.instantiate_default();

    suite.manage_incentive(
        other.clone(),
        IncentiveAction::Fill {
            params: IncentiveParams {
                lp_denom: lp_denom.clone(),
                start_epoch: None,
                preliminary_end_epoch: None,
                curve: None,
                incentive_asset: Coin {
                    denom: "ulab".to_string(),
                    amount: Uint128::new(4_000u128),
                },
                incentive_identifier: Some("incentive".to_string()),
            },
        },
        vec![coin(4_000, "ulab"), coin(1_000, "uwhale")],
        |result| {
            result.unwrap();
        },
    );

    // create a bunch of epochs to make the incentive expire
    for _ in 0..15 {
        suite.add_one_day().create_epoch(creator.clone(), |result| {
            result.unwrap();
        });
    }

    suite.manage_incentive(
        other.clone(),
        IncentiveAction::Fill {
            params: IncentiveParams {
                lp_denom: lp_denom.clone(),
                start_epoch: None,
                preliminary_end_epoch: None,
                curve: None,
                incentive_asset: Coin {
                    denom: "ulab".to_string(),
                    amount: Uint128::new(8_000u128),
                },
                incentive_identifier: Some("incentive".to_string()),
            },
        },
        vec![coin(8_000u128, "ulab")],
        |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();
            match err {
                ContractError::IncentiveAlreadyExpired { .. } => {}
                _ => {
                    panic!("Wrong error type, should return ContractError::IncentiveAlreadyExpired")
                }
            }
        },
    );
}

#[test]
fn test_emergency_withdrawal() {
    let lp_denom = "factory/pool/uLP".to_string();

    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_000u128, "uwhale".to_string()),
        coin(1_000_000_000u128, "ulab".to_string()),
        coin(1_000_000_000u128, "uosmo".to_string()),
        coin(1_000_000_000u128, lp_denom.clone()),
    ]);

    let other = suite.senders[1].clone();

    suite.instantiate_default();

    let whale_lair_addr = suite.whale_lair_addr.clone();

    suite
        .manage_incentive(
            other.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom.clone(),
                    start_epoch: None,
                    preliminary_end_epoch: None,
                    curve: None,
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(4_000u128),
                    },
                    incentive_identifier: Some("incentive".to_string()),
                },
            },
            vec![coin(4_000, "ulab"), coin(1_000, "uwhale")],
            |result| {
                result.unwrap();
            },
        )
        .manage_position(
            other.clone(),
            PositionAction::Fill {
                identifier: Some("other_position".to_string()),
                unlocking_duration: 86_400,
                receiver: None,
            },
            vec![coin(1_000, lp_denom.clone())],
            |result| {
                result.unwrap();
            },
        )
        .query_positions(other.clone(), Some(true), |result| {
            let positions = result.unwrap();
            assert_eq!(positions.positions.len(), 1);
            assert_eq!(
                positions.positions[0],
                Position {
                    identifier: "other_position".to_string(),
                    lp_asset: Coin {
                        denom: "factory/pool/uLP".to_string(),
                        amount: Uint128::new(1_000),
                    },
                    unlocking_duration: 86400,
                    open: true,
                    expiring_at: None,
                    receiver: other.clone(),
                }
            );
        })
        .query_balance(lp_denom.clone().to_string(), other.clone(), |balance| {
            assert_eq!(balance, Uint128::new(999_999_000));
        })
        .query_balance(
            lp_denom.clone().to_string(),
            whale_lair_addr.clone(),
            |balance| {
                assert_eq!(balance, Uint128::zero());
            },
        )
        .manage_position(
            other.clone(),
            PositionAction::Withdraw {
                identifier: "other_position".to_string(),
                emergency_unlock: Some(true),
            },
            vec![],
            |result| {
                result.unwrap();
            },
        )
        .query_balance(lp_denom.clone().to_string(), other.clone(), |balance| {
            //emergency unlock penalty is 10% of the position amount, so the user gets 1000 - 100 = 900
            assert_eq!(balance, Uint128::new(999_999_900));
        })
        .query_balance(
            lp_denom.clone().to_string(),
            whale_lair_addr.clone(),
            |balance| {
                assert_eq!(balance, Uint128::new(100));
            },
        );
}

#[test]
fn test_incentive_helper() {
    let lp_denom = "factory/pool/uLP".to_string();

    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_000u128, "uwhale".to_string()),
        coin(1_000_000_000u128, "ulab".to_string()),
        coin(1_000_000_000u128, "uosmo".to_string()),
        coin(1_000_000_000u128, lp_denom.clone()),
    ]);

    let creator = suite.creator();
    let other = suite.senders[1].clone();

    suite.instantiate_default();

    let incentive_manager_addr = suite.incentive_manager_addr.clone();
    let whale_lair_addr = suite.whale_lair_addr.clone();

    suite
        .manage_incentive(
            creator.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom.clone(),
                    start_epoch: None,
                    preliminary_end_epoch: None,
                    curve: None,
                    incentive_asset: Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(4_000u128),
                    },
                    incentive_identifier: Some("incentive".to_string()),
                },
            },
            vec![coin(3_000, "uwhale")],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::AssetMismatch { .. } => {}
                    _ => {
                        panic!("Wrong error type, should return ContractError::AssetMismatch")
                    }
                }
            },
        )
        .query_balance("uwhale".to_string(), creator.clone(), |balance| {
            assert_eq!(balance, Uint128::new(1_000_000_000));
        })
        .query_balance("uwhale".to_string(), whale_lair_addr.clone(), |balance| {
            assert_eq!(balance, Uint128::zero());
        })
        .query_balance(
            "uwhale".to_string(),
            incentive_manager_addr.clone(),
            |balance| {
                assert_eq!(balance, Uint128::zero());
            },
        )
        .manage_incentive(
            creator.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom.clone(),
                    start_epoch: None,
                    preliminary_end_epoch: None,
                    curve: None,
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(2_000u128),
                    },
                    incentive_identifier: Some("incentive".to_string()),
                },
            },
            vec![coin(2_000, "ulab"), coin(3_000, "uwhale")],
            |result| {
                result.unwrap();
            },
        )
        .query_balance("uwhale".to_string(), whale_lair_addr.clone(), |balance| {
            assert_eq!(balance, Uint128::new(1_000));
        })
        .query_balance(
            "uwhale".to_string(),
            incentive_manager_addr.clone(),
            |balance| {
                assert_eq!(balance, Uint128::zero());
            },
        )
        .query_balance("uwhale".to_string(), creator.clone(), |balance| {
            // got the excess of whale back
            assert_eq!(balance, Uint128::new(999_999_000));
        });

    suite.manage_incentive(
        other.clone(),
        IncentiveAction::Fill {
            params: IncentiveParams {
                lp_denom: lp_denom.clone(),
                start_epoch: None,
                preliminary_end_epoch: None,
                curve: None,
                incentive_asset: Coin {
                    denom: "ulab".to_string(),
                    amount: Uint128::new(2_000u128),
                },
                incentive_identifier: Some("underpaid_incentive".to_string()),
            },
        },
        vec![coin(2_000, "ulab"), coin(500, "uwhale")],
        |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();
            match err {
                ContractError::IncentiveFeeNotPaid { .. } => {}
                _ => {
                    panic!("Wrong error type, should return ContractError::IncentiveFeeNotPaid")
                }
            }
        },
    );
}

/// Complex test case with 4 incentives for 2 different LPs somewhat overlapping in time
/// Incentive 1 -> runs from epoch 12 to 16
/// Incentive 2 -> run from epoch 14 to 25
/// Incentive 3 -> runs from epoch 20 to 23
/// Incentive 4 -> runs from epoch 23 to 37
///
/// There are 3 users, creator, other and another
///
/// Locking tokens:
/// creator locks 35% of the LP tokens before incentive 1 starts
/// other locks 40% of the LP tokens before after incentive 1 starts and before incentive 2 starts
/// another locks 25% of the LP tokens after incentive 3 starts, before incentive 3 ends
///
/// Unlocking tokens:
/// creator never unlocks
/// other emergency unlocks mid-way through incentive 2
/// another partially unlocks mid-way through incentive 4
///
/// Verify users got rewards pro rata to their locked tokens
#[test]
fn test_multiple_incentives_and_positions() {
    let lp_denom_1 = "factory/pool1/uLP".to_string();
    let lp_denom_2 = "factory/pool2/uLP".to_string();

    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_000u128, "uwhale".to_string()),
        coin(1_000_000_000u128, "ulab".to_string()),
        coin(1_000_000_000u128, "uosmo".to_string()),
        coin(1_000_000_000u128, lp_denom_1.clone()),
        coin(1_000_000_000u128, lp_denom_2.clone()),
    ]);

    let creator = suite.creator();
    let other = suite.senders[1].clone();
    let another = suite.senders[2].clone();

    suite.instantiate_default();

    let incentive_manager_addr = suite.incentive_manager_addr.clone();
    let whale_lair_addr = suite.whale_lair_addr.clone();

    // create 4 incentives with 2 different LPs
    suite
        .add_hook(creator.clone(), incentive_manager_addr, vec![], |result| {
            result.unwrap();
        })
        .query_current_epoch(|result| {
            let epoch_response = result.unwrap();
            assert_eq!(epoch_response.epoch.id, 10);
        })
        .manage_incentive(
            creator.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom_1.clone(),
                    start_epoch: Some(12),
                    preliminary_end_epoch: Some(16),
                    curve: None,
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(80_000u128),
                    },
                    incentive_identifier: Some("incentive_1".to_string()),
                },
            },
            vec![coin(80_000u128, "ulab"), coin(1_000, "uwhale")],
            |result| {
                result.unwrap();
            },
        )
        .manage_incentive(
            creator.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom_1.clone(),
                    start_epoch: Some(14),
                    preliminary_end_epoch: Some(24),
                    curve: None,
                    incentive_asset: Coin {
                        denom: "uosmo".to_string(),
                        amount: Uint128::new(10_000u128),
                    },
                    incentive_identifier: Some("incentive_2".to_string()),
                },
            },
            vec![coin(10_000u128, "uosmo"), coin(1_000, "uwhale")],
            |result| {
                result.unwrap();
            },
        )
        .manage_incentive(
            other.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom_2.clone(),
                    start_epoch: Some(20),
                    preliminary_end_epoch: Some(23),
                    curve: None,
                    incentive_asset: Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(30_000u128),
                    },
                    incentive_identifier: Some("incentive_3".to_string()),
                },
            },
            vec![coin(31_000u128, "uwhale")],
            |result| {
                result.unwrap();
            },
        )
        .manage_incentive(
            other.clone(),
            IncentiveAction::Fill {
                params: IncentiveParams {
                    lp_denom: lp_denom_2.clone(),
                    start_epoch: Some(23),
                    preliminary_end_epoch: None,
                    curve: None,
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(70_000u128),
                    },
                    incentive_identifier: Some("incentive_4".to_string()),
                },
            },
            vec![coin(70_000u128, "ulab"), coin(1_000, "uwhale")],
            |result| {
                result.unwrap();
            },
        );

    // creator fills a position
    suite
        .manage_position(
            creator.clone(),
            PositionAction::Fill {
                identifier: Some("creator_pos_1".to_string()),
                unlocking_duration: 86_400,
                receiver: None,
            },
            vec![coin(35_000, lp_denom_1.clone())],
            |result| {
                result.unwrap();
            },
        )
        .manage_position(
            creator.clone(),
            PositionAction::Fill {
                identifier: Some("creator_pos_2".to_string()),
                unlocking_duration: 86_400,
                receiver: None,
            },
            vec![coin(70_000, lp_denom_2.clone())],
            |result| {
                result.unwrap();
            },
        );

    suite
        .add_one_epoch()
        .add_one_epoch()
        .add_one_epoch()
        .query_current_epoch(|result| {
            let epoch_response = result.unwrap();
            assert_eq!(epoch_response.epoch.id, 13);
        });

    // other fills a position
    suite
        .manage_position(
            other.clone(),
            PositionAction::Fill {
                identifier: Some("other_pos_1".to_string()),
                unlocking_duration: 86_400,
                receiver: None,
            },
            vec![coin(40_000, lp_denom_1.clone())],
            |result| {
                result.unwrap();
            },
        )
        .manage_position(
            other.clone(),
            PositionAction::Fill {
                identifier: Some("other_pos_2".to_string()),
                unlocking_duration: 86_400,
                receiver: None,
            },
            vec![coin(80_000, lp_denom_2.clone())],
            |result| {
                result.unwrap();
            },
        );

    suite
        .add_one_epoch()
        .add_one_epoch()
        .query_current_epoch(|result| {
            let epoch_response = result.unwrap();
            assert_eq!(epoch_response.epoch.id, 15);
        });

    suite
        .query_incentives(
            Some(IncentivesBy::Identifier("incentive_1".to_string())),
            None,
            None,
            |result| {
                let incentives_response = result.unwrap();
                assert_eq!(
                    incentives_response.incentives[0],
                    Incentive {
                        identifier: "incentive_1".to_string(),
                        owner: creator.clone(),
                        lp_denom: lp_denom_1.clone(),
                        incentive_asset: Coin {
                            denom: "ulab".to_string(),
                            amount: Uint128::new(80_000u128),
                        },
                        claimed_amount: Uint128::zero(),
                        emission_rate: Uint128::new(20_000),
                        curve: Curve::Linear,
                        start_epoch: 12u64,
                        preliminary_end_epoch: 16u64,
                        last_epoch_claimed: 11u64,
                    }
                );
            },
        )
        .query_balance("ulab".to_string(), creator.clone(), |balance| {
            assert_eq!(balance, Uint128::new(999_920_000));
        })
        .claim(creator.clone(), vec![], |result| {
            result.unwrap();
        })
        .query_balance("ulab".to_string(), creator.clone(), |balance| {
            assert_eq!(balance, Uint128::new(999_978_666));
        })
        .query_incentives(None, None, None, |result| {
            let incentives_response = result.unwrap();
            assert_eq!(
                incentives_response.incentives[0],
                Incentive {
                    identifier: "incentive_1".to_string(),
                    owner: creator.clone(),
                    lp_denom: lp_denom_1.clone(),
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(80_000u128),
                    },
                    claimed_amount: Uint128::new(58_666),
                    emission_rate: Uint128::new(20_000),
                    curve: Curve::Linear,
                    start_epoch: 12u64,
                    preliminary_end_epoch: 16u64,
                    last_epoch_claimed: 15u64,
                }
            );
            assert_eq!(
                incentives_response.incentives[1],
                Incentive {
                    identifier: "incentive_2".to_string(),
                    owner: creator.clone(),
                    lp_denom: lp_denom_1.clone(),
                    incentive_asset: Coin {
                        denom: "uosmo".to_string(),
                        amount: Uint128::new(10_000u128),
                    },
                    claimed_amount: Uint128::new(932),
                    emission_rate: Uint128::new(1_000),
                    curve: Curve::Linear,
                    start_epoch: 14u64,
                    preliminary_end_epoch: 24u64,
                    last_epoch_claimed: 15u64,
                }
            );
        });

    suite
        .add_one_epoch()
        .add_one_epoch()
        .add_one_epoch()
        .add_one_epoch()
        .query_current_epoch(|result| {
            let epoch_response = result.unwrap();
            assert_eq!(epoch_response.epoch.id, 19);
        });

    // other emergency unlocks mid-way incentive 2
    suite
        .query_balance("ulab".to_string(), other.clone(), |balance| {
            assert_eq!(balance, Uint128::new(999_930_000));
        })
        .query_balance("uosmo".to_string(), other.clone(), |balance| {
            assert_eq!(balance, Uint128::new(1_000_000_000));
        })
        .claim(other.clone(), vec![], |result| {
            result.unwrap();
        })
        .query_balance("ulab".to_string(), other.clone(), |balance| {
            assert_eq!(balance, Uint128::new(999_951_332));
        })
        .query_balance("uosmo".to_string(), other.clone(), |balance| {
            assert_eq!(balance, Uint128::new(1_000_003_198));
        })
        .query_incentives(None, None, None, |result| {
            let incentives_response = result.unwrap();
            assert_eq!(
                incentives_response.incentives[0],
                Incentive {
                    identifier: "incentive_1".to_string(),
                    owner: creator.clone(),
                    lp_denom: lp_denom_1.clone(),
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(80_000u128),
                    },
                    claimed_amount: Uint128::new(79_998u128), // exhausted
                    emission_rate: Uint128::new(20_000),
                    curve: Curve::Linear,
                    start_epoch: 12u64,
                    preliminary_end_epoch: 16u64,
                    last_epoch_claimed: 19u64,
                }
            );
            assert_eq!(
                incentives_response.incentives[1],
                Incentive {
                    identifier: "incentive_2".to_string(),
                    owner: creator.clone(),
                    lp_denom: lp_denom_1.clone(),
                    incentive_asset: Coin {
                        denom: "uosmo".to_string(),
                        amount: Uint128::new(10_000u128),
                    },
                    claimed_amount: Uint128::new(4_130),
                    emission_rate: Uint128::new(1_000),
                    curve: Curve::Linear,
                    start_epoch: 14u64,
                    preliminary_end_epoch: 24u64,
                    last_epoch_claimed: 19u64,
                }
            );
        })
        .manage_position(
            other.clone(),
            PositionAction::Withdraw {
                identifier: "other_pos_1".to_string(),
                emergency_unlock: Some(true),
            },
            vec![],
            |result| {
                result.unwrap();
            },
        )
        .manage_position(
            other.clone(),
            PositionAction::Withdraw {
                identifier: "other_pos_2".to_string(),
                emergency_unlock: Some(true),
            },
            vec![],
            |result| {
                result.unwrap();
            },
        )
        .query_balance(
            lp_denom_1.clone().to_string(),
            whale_lair_addr.clone(),
            |balance| {
                // 10% of the lp the user input initially
                assert_eq!(balance, Uint128::new(4_000));
            },
        )
        .query_balance(
            lp_denom_2.clone().to_string(),
            whale_lair_addr.clone(),
            |balance| {
                // 10% of the lp the user input initially
                assert_eq!(balance, Uint128::new(8_000));
            },
        );

    // at this point, other doesn't have any positions, and creator owns 100% of the weight

    suite.add_one_epoch().query_current_epoch(|result| {
        let epoch_response = result.unwrap();
        assert_eq!(epoch_response.epoch.id, 20);
    });

    // another fills a position
    suite.manage_position(
        another.clone(),
        PositionAction::Fill {
            identifier: Some("another_pos_1".to_string()),
            unlocking_duration: 15_778_476, // 6 months, should give him 5x multiplier
            receiver: None,
        },
        vec![coin(6_000, lp_denom_2.clone())],
        |result| {
            result.unwrap();
        },
    );

    // creator that had 100% now has ~70% of the weight, while another has ~30%
    suite
        .add_one_epoch()
        .add_one_epoch()
        .add_one_epoch()
        .add_one_epoch()
        .add_one_epoch()
        .add_one_epoch()
        .add_one_epoch()
        .add_one_epoch()
        .add_one_epoch()
        .add_one_epoch()
        .query_current_epoch(|result| {
            let epoch_response = result.unwrap();
            assert_eq!(epoch_response.epoch.id, 30);
        });

    suite
        .claim(creator.clone(), vec![], |result| {
            // creator claims from epoch 16 to 30
            // There's nothing to claim on incentive 1
            // On incentive 2, creator has a portion of the total weight until the epoch where other
            // triggered the emergency withdrawal. From that point (epoch 20) it has 100% of the weight
            // for lp_denom_1.
            // another never locked for lp_denom_1, so creator gets all the rewards for the incentive 2
            // from epoch 20 till it finishes at epoch 23
            result.unwrap();
        })
        .query_incentives(None, None, None, |result| {
            let incentives_response = result.unwrap();
            assert_eq!(
                incentives_response.incentives[0],
                Incentive {
                    identifier: "incentive_1".to_string(),
                    owner: creator.clone(),
                    lp_denom: lp_denom_1.clone(),
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(80_000u128),
                    },
                    claimed_amount: Uint128::new(79_998u128), // exhausted
                    emission_rate: Uint128::new(20_000),
                    curve: Curve::Linear,
                    start_epoch: 12u64,
                    preliminary_end_epoch: 16u64,
                    last_epoch_claimed: 19u64,
                }
            );
            assert_eq!(
                incentives_response.incentives[1],
                Incentive {
                    identifier: "incentive_2".to_string(),
                    owner: creator.clone(),
                    lp_denom: lp_denom_1.clone(),
                    incentive_asset: Coin {
                        denom: "uosmo".to_string(),
                        amount: Uint128::new(10_000u128),
                    },
                    claimed_amount: Uint128::new(9_994), // exhausted
                    emission_rate: Uint128::new(1_000),
                    curve: Curve::Linear,
                    start_epoch: 14u64,
                    preliminary_end_epoch: 24u64,
                    last_epoch_claimed: 30u64,
                }
            );
            assert_eq!(
                incentives_response.incentives[2],
                Incentive {
                    identifier: "incentive_3".to_string(),
                    owner: other.clone(),
                    lp_denom: lp_denom_2.clone(),
                    incentive_asset: Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(30_000u128),
                    },
                    claimed_amount: Uint128::new(24_000),
                    emission_rate: Uint128::new(10_000),
                    curve: Curve::Linear,
                    start_epoch: 20u64,
                    preliminary_end_epoch: 23u64,
                    last_epoch_claimed: 30u64,
                }
            );
            assert_eq!(
                incentives_response.incentives[3],
                Incentive {
                    identifier: "incentive_4".to_string(),
                    owner: other.clone(),
                    lp_denom: lp_denom_2.clone(),
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(70_000u128),
                    },
                    claimed_amount: Uint128::new(28_000),
                    emission_rate: Uint128::new(5_000),
                    curve: Curve::Linear,
                    start_epoch: 23u64,
                    preliminary_end_epoch: 37u64,
                    last_epoch_claimed: 30u64,
                }
            );
        })
        .claim(another.clone(), vec![], |result| {
            result.unwrap();
        })
        .query_incentives(None, None, None, |result| {
            let incentives_response = result.unwrap();
            assert_eq!(
                incentives_response.incentives[0],
                Incentive {
                    identifier: "incentive_1".to_string(),
                    owner: creator.clone(),
                    lp_denom: lp_denom_1.clone(),
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(80_000u128),
                    },
                    claimed_amount: Uint128::new(79_998u128), // exhausted
                    emission_rate: Uint128::new(20_000),
                    curve: Curve::Linear,
                    start_epoch: 12u64,
                    preliminary_end_epoch: 16u64,
                    last_epoch_claimed: 19u64,
                }
            );
            assert_eq!(
                incentives_response.incentives[1],
                Incentive {
                    identifier: "incentive_2".to_string(),
                    owner: creator.clone(),
                    lp_denom: lp_denom_1.clone(),
                    incentive_asset: Coin {
                        denom: "uosmo".to_string(),
                        amount: Uint128::new(10_000u128),
                    },
                    claimed_amount: Uint128::new(9_994), // exhausted
                    emission_rate: Uint128::new(1_000),
                    curve: Curve::Linear,
                    start_epoch: 14u64,
                    preliminary_end_epoch: 24u64,
                    last_epoch_claimed: 30u64,
                }
            );
            assert_eq!(
                incentives_response.incentives[2],
                Incentive {
                    identifier: "incentive_3".to_string(),
                    owner: other.clone(),
                    lp_denom: lp_denom_2.clone(),
                    incentive_asset: Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(30_000u128),
                    },
                    claimed_amount: Uint128::new(30_000), // exhausted
                    emission_rate: Uint128::new(10_000),
                    curve: Curve::Linear,
                    start_epoch: 20u64,
                    preliminary_end_epoch: 23u64,
                    last_epoch_claimed: 30u64,
                }
            );
            assert_eq!(
                incentives_response.incentives[3],
                Incentive {
                    identifier: "incentive_4".to_string(),
                    owner: other.clone(),
                    lp_denom: lp_denom_2.clone(),
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(70_000u128),
                    },
                    claimed_amount: Uint128::new(40_000),
                    emission_rate: Uint128::new(5_000),
                    curve: Curve::Linear,
                    start_epoch: 23u64,
                    preliminary_end_epoch: 37u64,
                    last_epoch_claimed: 30u64,
                }
            );
        });

    // another closes part of his position mid-way through incentive 4.
    // since the total weight was 100k and he unlocked 50% of his position,
    // the new total weight is 85k, so he gets 15k/85k of the rewards while creator gets the rest
    suite.manage_position(
        another.clone(),
        PositionAction::Close {
            identifier: "another_pos_1".to_string(),
            lp_asset: Some(coin(3_000, lp_denom_2.clone())),
        },
        vec![],
        |result| {
            result.unwrap();
        },
    );

    suite
        .add_one_epoch()
        .add_one_epoch()
        .add_one_epoch()
        .add_one_epoch()
        .add_one_epoch()
        .query_current_epoch(|result| {
            let epoch_response = result.unwrap();
            assert_eq!(epoch_response.epoch.id, 35);
        });

    suite
        .claim(creator.clone(), vec![], |result| {
            result.unwrap();
        })
        .query_incentives(None, None, None, |result| {
            let incentives_response = result.unwrap();
            assert_eq!(
                incentives_response.incentives[3],
                Incentive {
                    identifier: "incentive_4".to_string(),
                    owner: other.clone(),
                    lp_denom: lp_denom_2.clone(),
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(70_000u128),
                    },
                    claimed_amount: Uint128::new(60_585),
                    emission_rate: Uint128::new(5_000),
                    curve: Curve::Linear,
                    start_epoch: 23u64,
                    preliminary_end_epoch: 37u64,
                    last_epoch_claimed: 35u64,
                }
            );
        })
        .claim(another.clone(), vec![], |result| {
            result.unwrap();
        })
        .query_incentives(None, None, None, |result| {
            let incentives_response = result.unwrap();
            assert_eq!(
                incentives_response.incentives[3],
                Incentive {
                    identifier: "incentive_4".to_string(),
                    owner: other.clone(),
                    lp_denom: lp_denom_2.clone(),
                    incentive_asset: Coin {
                        denom: "ulab".to_string(),
                        amount: Uint128::new(70_000u128),
                    },
                    claimed_amount: Uint128::new(64_995),
                    emission_rate: Uint128::new(5_000),
                    curve: Curve::Linear,
                    start_epoch: 23u64,
                    preliminary_end_epoch: 37u64,
                    last_epoch_claimed: 35u64,
                }
            );
        });

    // now the epochs go by, the incentive expires and the creator withdraws the rest of the incentive

    suite
        .add_one_epoch()
        .add_one_epoch()
        .add_one_epoch()
        .add_one_epoch()
        .add_one_epoch()
        .query_current_epoch(|result| {
            let epoch_response = result.unwrap();
            assert_eq!(epoch_response.epoch.id, 40);
        });

    suite.manage_incentive(
        creator.clone(),
        IncentiveAction::Close {
            incentive_identifier: "incentive_4".to_string(),
        },
        vec![],
        |result| {
            result.unwrap();
        },
    );
}
