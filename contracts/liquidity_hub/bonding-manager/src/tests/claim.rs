use std::collections::VecDeque;

use cosmwasm_std::{coin, Uint64};
use white_whale_std::fee::{Fee, PoolFee};
use white_whale_std::pool_network::asset::MINIMUM_LIQUIDITY_AMOUNT;

use crate::tests::suite::TestingSuite;
use crate::tests::test_helpers;
use cosmwasm_std::{coins, Coin, Decimal, Timestamp, Uint128};

use crate::ContractError;
use white_whale_std::bonding_manager::{BondedResponse, BondingWeightResponse, Epoch, GlobalIndex};

use super::test_helpers::get_epochs;

#[test]
fn test_claimable_epochs() {
    let mut robot = TestingSuite::default();
    let grace_period = Uint64::new(21);
    let creator = robot.sender.clone();

    let asset_denoms = vec!["uwhale".to_string(), "uusdc".to_string()];

    #[cfg(not(feature = "osmosis"))]
    let pool_fees = PoolFee {
        protocol_fee: Fee {
            share: Decimal::from_ratio(1u128, 100u128),
        },
        swap_fee: Fee {
            share: Decimal::from_ratio(1u128, 100u128),
        },
        burn_fee: Fee {
            share: Decimal::zero(),
        },
        extra_fees: vec![],
    };

    robot
        .instantiate_default()
        .fast_forward(259_200)
        .create_epoch(|result| {
            result.unwrap();
        })
        .create_pair(
            creator.clone(),
            asset_denoms.clone(),
            pool_fees.clone(),
            white_whale_std::pool_manager::PoolType::ConstantProduct,
            Some("whale-uusdc".to_string()),
            vec![coin(1000, "uwhale")],
            |result| {
                result.unwrap();
            },
        )
        .provide_liquidity(
            creator.clone(),
            "whale-uusdc".to_string(),
            vec![
                Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::from(1_000_000_000u128),
                },
                Coin {
                    denom: "uusdc".to_string(),
                    amount: Uint128::from(1_000_000_000u128),
                },
            ],
            |result| {
                // Ensure we got 999_000 in the response which is 1mil less the initial liquidity amount
                assert!(result.unwrap().events.iter().any(|event| {
                    event.attributes.iter().any(|attr| {
                        attr.key == "share"
                            && attr.value
                                == (Uint128::from(1_000_000_000u128) - MINIMUM_LIQUIDITY_AMOUNT)
                                    .to_string()
                    })
                }));
            },
        )
        .swap(
            creator.clone(),
            coin(1_000u128, "uusdc"),
            "uwhale".to_string(),
            None,
            None,
            None,
            "whale-uusdc".to_string(),
            vec![Coin {
                denom: "uusdc".to_string(),
                amount: Uint128::from(1_000u128),
            }],
            |result| {
                result.unwrap();
            },
        )
        .swap(
            creator.clone(),
            coin(1_000u128, "uwhale"),
            "uusdc".to_string(),
            None,
            None,
            None,
            "whale-uusdc".to_string(),
            vec![Coin {
                denom: "uwhale".to_string(),
                amount: Uint128::from(1_000u128),
            }],
            |result| {
                result.unwrap();
            },
        )
        .create_epoch(|result| {
            result.unwrap();
        })
        .swap(
            creator.clone(),
            coin(2_000u128, "uusdc"),
            "uwhale".to_string(),
            None,
            None,
            None,
            "whale-uusdc".to_string(),
            vec![Coin {
                denom: "uusdc".to_string(),
                amount: Uint128::from(2_000u128),
            }],
            |result| {
                result.unwrap();
            },
        )
        .swap(
            creator.clone(),
            coin(2_000u128, "uwhale"),
            "uusdc".to_string(),
            None,
            None,
            None,
            "whale-uusdc".to_string(),
            vec![Coin {
                denom: "uwhale".to_string(),
                amount: Uint128::from(2_000u128),
            }],
            |result| {
                result.unwrap();
            },
        );

    let expected_epochs = vec![Epoch {
        id: Uint64::new(2u64),
        start_time: Timestamp::from_nanos(1571970229879305533),
        total: vec![coin(1009u128, "uwhale")],
        available: vec![coin(1009u128, "uwhale")],
        claimed: vec![],
        global_index: GlobalIndex {
            bonded_amount: Default::default(),
            bonded_assets: vec![],
            timestamp: Timestamp::from_nanos(1572056619879305533),
            weight: Default::default(),
        },
    }];

    robot.query_claimable_epochs(None, |res| {
        let (_, epochs) = res.unwrap();
        assert_eq!(epochs.len(), expected_epochs.len());

        for (index, epoch) in epochs.iter().enumerate() {
            assert_eq!(expected_epochs[index], epoch.clone());
        }
    });
}

#[test]
fn test_claim_successfully() {
    let mut robot = TestingSuite::default();
    let sender = robot.sender.clone();
    let another_sender = robot.another_sender.clone();
    let asset_infos = vec!["uwhale".to_string(), "uusdc".to_string()];

    // Default Pool fees white_whale_std::pool_network::pair::PoolFee
    // Protocol fee is 0.01% and swap fee is 0.02% and burn fee is 0%
    #[cfg(not(feature = "osmosis"))]
    let pool_fees = PoolFee {
        protocol_fee: Fee {
            share: Decimal::from_ratio(1u128, 100u128),
        },
        swap_fee: Fee {
            share: Decimal::from_ratio(1u128, 100u128),
        },
        burn_fee: Fee {
            share: Decimal::zero(),
        },
        extra_fees: vec![],
    };

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
        .fast_forward(100_000u64)
        .create_epoch(|result| {
            result.unwrap();
        })
        .assert_bonding_weight_response(
            sender.to_string(),
            BondingWeightResponse {
                address: sender.to_string(),
                weight: Uint128::new(100_001_000u128),
                global_weight: Uint128::new(100_001_000u128),
                share: Decimal::one(),
                timestamp: Timestamp::from_nanos(1571897419879305533u64),
            },
        )
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
        .query_total_bonded(|res| {
            let bonded_response = res.unwrap().1;
            assert_eq!(
                bonded_response,
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
                }
            )
        });

    robot.query_claimable_epochs_live(Some(sender.clone()), |res| {
        let (_, epochs) = res.unwrap();
        assert_eq!(epochs.len(), 0);
    });

    println!("-------");
    robot
        .create_pair(
            sender.clone(),
            asset_infos.clone(),
            pool_fees.clone(),
            white_whale_std::pool_manager::PoolType::ConstantProduct,
            Some("whale-uusdc".to_string()),
            vec![coin(1000, "uwhale")],
            |result| {
                result.unwrap();
            },
        )
        .provide_liquidity(
            sender.clone(),
            "whale-uusdc".to_string(),
            vec![
                Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::from(1000000000u128),
                },
                Coin {
                    denom: "uusdc".to_string(),
                    amount: Uint128::from(1000000000u128),
                },
            ],
            |result| {
                // Ensure we got 999_000 in the response which is 1mil less the initial liquidity amount
                assert!(result.unwrap().events.iter().any(|event| {
                    event.attributes.iter().any(|attr| {
                        attr.key == "share"
                            && attr.value
                                == (Uint128::from(1000000000u128) - MINIMUM_LIQUIDITY_AMOUNT)
                                    .to_string()
                    })
                }));
            },
        )
        .swap(
            sender.clone(),
            coin(1_000u128, "uusdc"),
            "uwhale".to_string(),
            None,
            None,
            None,
            "whale-uusdc".to_string(),
            vec![Coin {
                denom: "uusdc".to_string(),
                amount: Uint128::from(1_000u128),
            }],
            |result| {
                result.unwrap();
            },
        )
        .swap(
            sender.clone(),
            coin(1_000u128, "uwhale"),
            "uusdc".to_string(),
            None,
            None,
            None,
            "whale-uusdc".to_string(),
            vec![Coin {
                denom: "uwhale".to_string(),
                amount: Uint128::from(1_000u128),
            }],
            |result| {
                result.unwrap();
            },
        )
        .fast_forward(90_000)
        .create_epoch(|result| {
            result.unwrap();
        })
        .swap(
            sender.clone(),
            coin(1_000u128, "uusdc"),
            "uwhale".to_string(),
            None,
            None,
            None,
            "whale-uusdc".to_string(),
            vec![Coin {
                denom: "uusdc".to_string(),
                amount: Uint128::from(1_000u128),
            }],
            |result| {
                result.unwrap();
            },
        )
        .swap(
            sender.clone(),
            coin(1_000u128, "uwhale"),
            "uusdc".to_string(),
            None,
            None,
            None,
            "whale-uusdc".to_string(),
            vec![Coin {
                denom: "uwhale".to_string(),
                amount: Uint128::from(1_000u128),
            }],
            |result| {
                result.unwrap();
            },
        );

    robot.query_claimable_epochs_live(None, |res| {
        let (_, epochs) = res.unwrap();
        assert_eq!(epochs.len(), 1);
    });
    robot.query_claimable_epochs_live(Some(sender.clone()), |res| {
        let (_, epochs) = res.unwrap();
        assert_eq!(epochs.len(), 1);
    });

    robot.claim(sender.clone(), |res| {
        let result = res.unwrap();
        assert!(result.events.iter().any(|event| {
            event
                .attributes
                .iter()
                .any(|attr| attr.key == "amount" && attr.value == "571uwhale")
        }));
    });

    robot.claim(another_sender.clone(), |result| {
        let err = result.unwrap_err().downcast::<ContractError>().unwrap();
        match err {
            ContractError::NothingToClaim => {}
            _ => {
                panic!("Wrong error type, should return ContractError::NothingToClaim")
            }
        }
    });

    robot
        .bond(
            another_sender.clone(),
            Coin {
                denom: "ampWHALE".to_string(),
                amount: Uint128::new(9_000u128),
            },
            &coins(9_000u128, "ampWHALE"),
            |res| {
                res.unwrap();
            },
        )
        .assert_bonded_response(
            another_sender.to_string(),
            BondedResponse {
                total_bonded: Uint128::new(9_000u128),
                bonded_assets: vec![Coin {
                    denom: "ampWHALE".to_string(),
                    amount: Uint128::new(9_000u128),
                }],
                first_bonded_epoch_id: Uint64::new(3u64),
            },
        );

    robot
        .fast_forward(100_000)
        .create_epoch(|result| {
            result.unwrap();
            println!("*****");
        })
        .swap(
            another_sender.clone(),
            coin(1_000u128, "uusdc"),
            "uwhale".to_string(),
            None,
            None,
            None,
            "whale-uusdc".to_string(),
            vec![Coin {
                denom: "uusdc".to_string(),
                amount: Uint128::from(1_000u128),
            }],
            |result| {
                result.unwrap();
            },
        )
        .swap(
            another_sender.clone(),
            coin(1_000u128, "uwhale"),
            "uusdc".to_string(),
            None,
            None,
            None,
            "whale-uusdc".to_string(),
            vec![Coin {
                denom: "uwhale".to_string(),
                amount: Uint128::from(1_000u128),
            }],
            |result| {
                result.unwrap();
            },
        )
        .swap(
            sender.clone(),
            coin(5_000u128, "uusdc"),
            "uwhale".to_string(),
            None,
            None,
            None,
            "whale-uusdc".to_string(),
            vec![Coin {
                denom: "uusdc".to_string(),
                amount: Uint128::from(5_000u128),
            }],
            |result| {
                result.unwrap();
            },
        )
        .swap(
            sender.clone(),
            coin(5_000u128, "uwhale"),
            "uusdc".to_string(),
            None,
            None,
            None,
            "whale-uusdc".to_string(),
            vec![Coin {
                denom: "uwhale".to_string(),
                amount: Uint128::from(5_000u128),
            }],
            |result| {
                result.unwrap();
            },
        );

    robot.query_claimable_epochs_live(Some(another_sender.clone()), |res| {
        let (_, epochs) = res.unwrap();
        assert_eq!(epochs.len(), 1);
    });
}
