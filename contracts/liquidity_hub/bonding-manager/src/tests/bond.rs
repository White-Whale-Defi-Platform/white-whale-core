use cosmwasm_std::{coins, Coin, Decimal, Timestamp, Uint128};

use white_whale_std::bonding_manager::{BondedResponse, BondingWeightResponse};

use crate::tests::suite::TestingSuite;

use super::test_helpers::get_epochs;

#[test]
fn test_bond_successfully() {
    let mut robot = TestingSuite::default();
    let sender = robot.sender.clone();
    let another_sender = robot.another_sender.clone();
    get_epochs();
    robot
        .instantiate_default()
        .bond(
            sender.clone(),
            Coin {
                denom: "ampWHALE".to_string(),
                amount: Uint128::new(1_000u128),
            },
            &coins(1_000u128, "ampWHALE"),
            |_res| {},
        )
        .assert_bonded_response(
            sender.to_string(),
            BondedResponse {
                total_bonded: Uint128::new(1_000u128),
                bonded_assets: vec![Coin {
                    denom: "ampWHALE".to_string(),
                    amount: Uint128::new(1_000u128),
                }],
                first_bonded_epoch_id: Default::default(),
            },
        )
        .fast_forward(10u64)
        .assert_bonding_weight_response(
            sender.to_string(),
            BondingWeightResponse {
                address: sender.to_string(),
                weight: Uint128::new(11_000u128),
                global_weight: Uint128::new(11_000u128),
                share: Decimal::one(),
                timestamp: Timestamp::from_nanos(1571797429879305533u64),
            },
        )
        .fast_forward(10u64)
        .bond(
            sender.clone(),
            Coin {
                denom: "bWHALE".to_string(),
                amount: Uint128::new(3_000u128),
            },
            &coins(3_000u128, "bWHALE"),
            |_res| {},
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
                first_bonded_epoch_id: Default::default(),
            },
        )
        .fast_forward(10u64)
        // .assert_bonding_weight_response(
        //     sender.to_string(),
        //     BondingWeightResponse {
        //         address: sender.to_string(),
        //         weight: Uint128::new(64_000u128),
        //         global_weight: Uint128::new(64_000u128),
        //         share: Decimal::one(),
        //         timestamp: Timestamp::from_nanos(1571797449879305533u64),
        //     },
        // )
        .bond(
            another_sender.clone(),
            Coin {
                denom: "ampWHALE".to_string(),
                amount: Uint128::new(5_000u128),
            },
            &coins(5_000u128, "ampWHALE"),
            |_res| {},
        )
        .fast_forward(10u64)
        .assert_bonding_weight_response(
            sender.to_string(),
            BondingWeightResponse {
                address: sender.to_string(),
                weight: Uint128::new(104_000u128),
                global_weight: Uint128::new(269_000u128),
                share: Decimal::from_ratio(104_000u128, 269_000u128),
                timestamp: Timestamp::from_nanos(1571797459879305533u64),
            },
        )
        .assert_bonding_weight_response(
            another_sender.to_string(),
            BondingWeightResponse {
                address: another_sender.to_string(),
                weight: Uint128::new(55_000u128),
                global_weight: Uint128::new(269_000u128),
                share: Decimal::from_ratio(55_000u128, 269_000u128),
                timestamp: Timestamp::from_nanos(1571797459879305533u64),
            },
        )
        .query_total_bonded(|res| {
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
                    first_bonded_epoch_id: Default::default(),
                }
            )
        });
}
