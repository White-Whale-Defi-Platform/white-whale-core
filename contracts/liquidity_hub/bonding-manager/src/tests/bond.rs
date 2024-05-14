use cosmwasm_std::{coins, Coin, Decimal, Uint128};
use std::cell::RefCell;

use white_whale_std::bonding_manager::{BondedResponse, BondingWeightResponse, GlobalIndex};

use crate::tests::suite::TestingSuite;
use crate::ContractError;

#[test]
fn test_bond_successfully() {
    let mut suite = TestingSuite::default();
    let sender = suite.sender.clone();
    let another_sender = suite.another_sender.clone();

    let global_index = RefCell::new(GlobalIndex::default());

    suite
        .instantiate_default()
        .bond(
            sender.clone(),
            Coin {
                denom: "ampWHALE".to_string(),
                amount: Uint128::new(1_000u128),
            },
            &coins(1_000u128, "ampWHALE"),
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    // no epochs has been created yet
                    ContractError::Unauthorized => {}
                    _ => {
                        panic!("Wrong error type, should return ContractError::Unauthorized")
                    }
                }
            },
        )
        .assert_bonded_response(
            sender.to_string(),
            BondedResponse {
                total_bonded: Default::default(),
                bonded_assets: Default::default(),
                first_bonded_epoch_id: None,
            },
        );

    suite
        .add_one_day()
        // created epoch 1
        .create_new_epoch()
        .query_global_index(Some(1u64), |res| {
            let gi = res.unwrap().1;
            *global_index.borrow_mut() = gi.clone();
            println!("gi 1:: {:?}", gi);
        })
        .bond(
            sender.clone(),
            Coin {
                denom: "ampWHALE".to_string(),
                amount: Uint128::new(1_000u128),
            },
            &coins(1_000u128, "ampWHALE"),
            |result| {
                result.unwrap();
            },
        )
        .assert_bonded_response(
            sender.to_string(),
            BondedResponse {
                total_bonded: Uint128::new(1_000u128),
                bonded_assets: vec![Coin {
                    denom: "ampWHALE".to_string(),
                    amount: Uint128::new(1_000u128),
                }],
                first_bonded_epoch_id: Some(1u64),
            },
        )
        .assert_bonding_weight_response(
            sender.to_string(),
            Some(1u64),
            Some(global_index.clone().into_inner()),
            BondingWeightResponse {
                address: sender.to_string(),
                weight: Uint128::zero(),
                global_weight: Uint128::zero(), // because the snapshot was taken at the beginning of the epoch
                share: Decimal::zero(),
                epoch_id: 1u64,
            },
        );

    suite
        .fast_forward(43_200u64)
        .bond(
            sender.clone(),
            Coin {
                denom: "bWHALE".to_string(),
                amount: Uint128::new(3_000u128),
            },
            &coins(3_000u128, "bWHALE"),
            |result| {
                result.unwrap();
            },
        )
        .assert_bonded_response(
            sender.to_string(),
            BondedResponse {
                total_bonded: Uint128::new(4_000u128),
                bonded_assets: vec![
                    Coin {
                        denom: "ampWHALE".to_string(),
                        amount: Uint128::new(1_000u128),
                    },
                    Coin {
                        denom: "bWHALE".to_string(),
                        amount: Uint128::new(3_000u128),
                    },
                ],
                first_bonded_epoch_id: Some(1u64),
            },
        );

    suite
        .add_one_day()
        // epoch 2
        .create_new_epoch()
        .query_global_index(Some(2u64), |res| {
            let gi = res.unwrap().1;
            println!("gi 2:: {:?}", gi);
            *global_index.borrow_mut() = gi.clone();
        });

    suite
        .query_weight(
            sender.to_string(),
            Some(2u64),
            Some(global_index.clone().into_inner()),
            |res| {
                let bonded_response = res.unwrap().1;
                println!("bonded_response 1:: {:?}", bonded_response);
            },
        )
        .bond(
            sender.clone(),
            Coin {
                denom: "bWHALE".to_string(),
                amount: Uint128::new(1_000u128),
            },
            &coins(1_000u128, "bWHALE"),
            |result| {
                result.unwrap();
            },
        )
        .assert_bonded_response(
            sender.to_string(),
            BondedResponse {
                total_bonded: Uint128::new(5_000u128),
                bonded_assets: vec![
                    Coin {
                        denom: "ampWHALE".to_string(),
                        amount: Uint128::new(1_000u128),
                    },
                    Coin {
                        denom: "bWHALE".to_string(),
                        amount: Uint128::new(4_000u128),
                    },
                ],
                first_bonded_epoch_id: Some(1u64),
            },
        );

    println!(
        "herrreeee global_index:: {:?}",
        global_index.clone().into_inner()
    );

    suite
        .assert_bonding_weight_response(
            sender.to_string(),
            Some(2u64),
            Some(global_index.clone().into_inner()),
            BondingWeightResponse {
                address: sender.to_string(),
                weight: Uint128::new(6_000u128),
                global_weight: Uint128::new(8_000u128),
                share: Decimal::from_ratio(6_000u128, 8_000u128),
                epoch_id: 2u64,
            },
        )
        .query_weight(sender.to_string(), Some(2u64), None, |res| {
            let bonded_response = res.unwrap().1;
            println!("bonded_response 2:: {:?}", bonded_response);
        });

    suite
        .query_bonded(None, |res| {
            let bonded_response = res.unwrap().1;
            assert_eq!(
                bonded_response,
                BondedResponse {
                    total_bonded: Uint128::new(5_000u128),
                    bonded_assets: vec![
                        Coin {
                            denom: "ampWHALE".to_string(),
                            amount: Uint128::new(1_000u128),
                        },
                        Coin {
                            denom: "bWHALE".to_string(),
                            amount: Uint128::new(4_000u128),
                        },
                    ],
                    first_bonded_epoch_id: None,
                }
            )
        })
        .bond(
            another_sender.clone(),
            Coin {
                denom: "ampWHALE".to_string(),
                amount: Uint128::new(5_000u128),
            },
            &coins(5_000u128, "ampWHALE"),
            |_res| {},
        )
        .assert_bonded_response(
            another_sender.to_string(),
            BondedResponse {
                total_bonded: Uint128::new(5_000u128),
                bonded_assets: vec![Coin {
                    denom: "ampWHALE".to_string(),
                    amount: Uint128::new(5_000u128),
                }],
                first_bonded_epoch_id: Some(2u64),
            },
        )
        .assert_bonding_weight_response(
            another_sender.to_string(),
            Some(2u64),
            Some(global_index.clone().into_inner()),
            BondingWeightResponse {
                address: another_sender.to_string(),
                weight: Uint128::new(5_000u128),
                global_weight: Uint128::new(15_000u128),
                share: Decimal::from_ratio(5_000u128, 15_000u128),
                epoch_id: 2u64,
            },
        )
        .query_bonded(None, |res| {
            let bonded_response = res.unwrap().1;
            assert_eq!(
                bonded_response,
                BondedResponse {
                    total_bonded: Uint128::new(10_000u128),
                    bonded_assets: vec![
                        Coin {
                            denom: "ampWHALE".to_string(),
                            amount: Uint128::new(6_000u128),
                        },
                        Coin {
                            denom: "bWHALE".to_string(),
                            amount: Uint128::new(4_000u128),
                        },
                    ],
                    first_bonded_epoch_id: None,
                }
            )
        })
        .query_weight(sender.to_string(), Some(2u64), None, |res| {
            let bonded_response = res.unwrap().1;
            println!("bonded_response sender:: {:?}", bonded_response);
        })
        .query_weight(another_sender.to_string(), Some(2u64), None, |res| {
            let bonded_response = res.unwrap().1;
            println!("bonded_response another_sender:: {:?}", bonded_response);
        });

    suite
        .add_one_day()
        .create_new_epoch()
        .query_global_index(Some(3u64), |res| {
            let gi = res.unwrap().1;
            *global_index.borrow_mut() = gi.clone();
            println!("gi:: {:?}", gi);
        })
        .query_weight(sender.to_string(), Some(3u64), None, |res| {
            let bonded_response = res.unwrap().1;
            println!("bonded_response sender again:: {:?}", bonded_response);
        })
        .query_weight(another_sender.to_string(), Some(3u64), None, |res| {
            let bonded_response = res.unwrap().1;
            println!(
                "bonded_response another_sender again:: {:?}",
                bonded_response
            );
        });

    suite.assert_bonding_weight_response(
        another_sender.to_string(),
        Some(3u64),
        Some(global_index.clone().into_inner()),
        BondingWeightResponse {
            address: another_sender.to_string(),
            weight: Uint128::new(10_000u128),
            global_weight: Uint128::new(25_000u128),
            share: Decimal::from_ratio(10_000u128, 25_000u128),
            epoch_id: 3u64,
        },
    );

    suite
        .bond(
            another_sender.clone(),
            Coin {
                denom: "ampWHALE".to_string(),
                amount: Uint128::new(2_000u128),
            },
            &coins(2_000u128, "bWHALE"),
            |_res| {},
        )
        .assert_bonded_response(
            another_sender.to_string(),
            BondedResponse {
                total_bonded: Uint128::new(7_000u128),
                bonded_assets: vec![
                    Coin {
                        denom: "ampWHALE".to_string(),
                        amount: Uint128::new(5_000u128),
                    },
                    Coin {
                        denom: "bWHALE".to_string(),
                        amount: Uint128::new(2_000u128),
                    },
                ],
                first_bonded_epoch_id: Some(2u64),
            },
        )
        .assert_bonding_weight_response(
            another_sender.to_string(),
            Some(3u64),
            Some(global_index.clone().into_inner()),
            BondingWeightResponse {
                address: another_sender.to_string(),
                weight: Uint128::new(12_000u128),
                global_weight: Uint128::new(29_000u128),
                share: Decimal::from_ratio(12_000u128, 29_000u128),
                epoch_id: 3u64,
            },
        )
        .assert_bonding_weight_response(
            sender.to_string(),
            Some(3u64),
            Some(global_index.clone().into_inner()),
            BondingWeightResponse {
                address: sender.to_string(),
                weight: Uint128::new(15_000u128),
                global_weight: Uint128::new(29_000u128),
                share: Decimal::from_ratio(15_000u128, 29_000u128),
                epoch_id: 3u64,
            },
        );
}
