use cosmwasm_std::{coins, Coin, Decimal, Timestamp, Uint128, Uint64};

use crate::ContractError;
use white_whale_std::bonding_manager::{BondedResponse, BondingWeightResponse};

use crate::tests::suite::TestingSuite;

#[test]
fn test_bond_successfully() {
    let mut suite = TestingSuite::default();
    let sender = suite.sender.clone();
    let another_sender = suite.another_sender.clone();

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
        .create_new_epoch()
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
                first_bonded_epoch_id: Some(Uint64::new(1u64)),
            },
        )
        .assert_bonding_weight_response(
            sender.to_string(),
            BondingWeightResponse {
                address: sender.to_string(),
                weight: Uint128::new(1_000u128),
                global_weight: Uint128::new(1_000u128),
                share: Decimal::one(),
                timestamp: Timestamp::from_nanos(1571883819879305533u64),
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
                first_bonded_epoch_id: Some(Uint64::new(1u64)),
            },
        );

    suite
        .add_one_day()
        .create_new_epoch()
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
                first_bonded_epoch_id: Some(Uint64::new(2u64)),
            },
        )
        .assert_bonding_weight_response(
            another_sender.to_string(),
            BondingWeightResponse {
                address: another_sender.to_string(),
                weight: Uint128::new(5_000u128),
                global_weight: Uint128::new(950_409_000u128),
                share: Decimal::from_ratio(5_000u128, 950_409_000u128),
                timestamp: Timestamp::from_nanos(1572013419879305533u64),
            },
        )
        .query_bonded(None, |res| {
            let bonded_response = res.unwrap().1;
            assert_eq!(
                bonded_response,
                BondedResponse {
                    total_bonded: Uint128::new(9_000u128),
                    bonded_assets: vec![
                        Coin {
                            denom: "ampWHALE".to_string(),
                            amount: Uint128::new(6_000u128),
                        },
                        Coin {
                            denom: "bWHALE".to_string(),
                            amount: Uint128::new(3_000u128),
                        },
                    ],
                    first_bonded_epoch_id: None,
                }
            )
        });

    suite
        .add_one_day()
        .assert_bonding_weight_response(
            sender.to_string(),
            BondingWeightResponse {
                address: sender.to_string(),
                weight: Uint128::new(734_404_000u128),
                global_weight: Uint128::new(1_728_009_000u128),
                share: Decimal::from_ratio(734_404_000u128, 1_728_009_000u128),
                timestamp: Timestamp::from_nanos(1572099819879305533u64),
            },
        )
        .assert_bonding_weight_response(
            another_sender.to_string(),
            BondingWeightResponse {
                address: another_sender.to_string(),
                weight: Uint128::new(432_005_000u128),
                global_weight: Uint128::new(1_728_009_000u128),
                share: Decimal::from_ratio(432_005_000u128, 1_728_009_000u128),
                timestamp: Timestamp::from_nanos(1572099819879305533u64),
            },
        )
        .query_bonded(None, |result| {
            let res = result.unwrap().1;
            println!("{:?}", res);
        });
}
