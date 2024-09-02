use cosmwasm_std::{coin, coins, Coin, Decimal, Uint128};

use white_whale_std::bonding_manager::{Bond, BondedResponse, GlobalIndex, UnbondingResponse};
use white_whale_std::fee::{Fee, PoolFee};
use white_whale_std::pool_network::asset::MINIMUM_LIQUIDITY_AMOUNT;

use crate::tests::suite::TestingSuite;
use crate::ContractError;

#[test]
fn test_queries() {
    let mut suite = TestingSuite::default();
    let creator = suite.senders[0].clone();

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

    #[cfg(feature = "osmosis")]
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
        osmosis_fee: Fee {
            share: Decimal::permille(1),
        },
    };

    suite
        .instantiate_default()
        .bond(creator.clone(), &coins(1_000u128, "ampWHALE"), |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();
            // the system has not been initialized
            match err {
                ContractError::Unauthorized { .. } => {}
                _ => panic!("Wrong error type, should return ContractError::Unauthorized"),
            }
        });

    suite
        .add_one_day()
        // epoch 1
        .create_new_epoch()
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
        .add_one_day()
        // epoch 2
        .create_new_epoch()
        .bond(creator.clone(), &coins(1_000u128, "ampWHALE"), |res| {
            res.unwrap();
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

    // epoch 3
    suite.add_one_day().create_new_epoch();

    suite
        .query_global_index(None, |result| {
            let global_index = result.unwrap().1;
            assert_eq!(
                global_index,
                GlobalIndex {
                    epoch_id: 3,
                    bonded_amount: Uint128::new(1_000),
                    bonded_assets: vec![Coin {
                        denom: "ampWHALE".to_string(),
                        amount: Uint128::from(1_000u128),
                    }],
                    last_updated: 2,
                    last_weight: Uint128::from(1_000u128),
                }
            );
        })
        .query_global_index(Some(3u64), |result| {
            let global_index = result.unwrap().1;
            assert_eq!(
                global_index,
                GlobalIndex {
                    epoch_id: 3,
                    bonded_amount: Uint128::new(1_000),
                    bonded_assets: vec![Coin {
                        denom: "ampWHALE".to_string(),
                        amount: Uint128::from(1_000u128),
                    }],
                    last_updated: 2,
                    last_weight: Uint128::from(1_000u128),
                }
            );
        })
        .query_global_index(Some(2u64), |result| {
            let global_index = result.unwrap().1;
            assert_eq!(
                global_index,
                GlobalIndex {
                    epoch_id: 2,
                    bonded_amount: Default::default(),
                    bonded_assets: vec![],
                    last_updated: 2,
                    last_weight: Default::default(),
                }
            );
        })
        .query_global_index(Some(1u64), |result| {
            let global_index = result.unwrap().1;
            assert_eq!(
                global_index,
                GlobalIndex {
                    epoch_id: 1,
                    bonded_amount: Default::default(),
                    bonded_assets: vec![],
                    last_updated: 1,
                    last_weight: Default::default(),
                }
            );
        })
        .query_bonded(None, |result| {
            let bonded_response = result.unwrap().1;
            assert_eq!(
                bonded_response,
                BondedResponse {
                    total_bonded: Uint128::new(1_000u128),
                    bonded_assets: vec![Coin {
                        denom: "ampWHALE".to_string(),
                        amount: Uint128::from(1_000u128),
                    }],
                }
            );
        });

    suite.claim(creator.clone(), |result| {
        result.unwrap();
    });

    suite
        .unbond(creator.clone(), coin(100u128, "ampWHALE"), |result| {
            result.unwrap();
        })
        .fast_forward(1_000)
        .unbond(creator.clone(), coin(200u128, "ampWHALE"), |result| {
            result.unwrap();
        })
        .fast_forward(1_000)
        .unbond(creator.clone(), coin(300u128, "ampWHALE"), |result| {
            result.unwrap();
        })
        .fast_forward(1_000)
        .unbond(creator.clone(), coin(400u128, "ampWHALE"), |result| {
            result.unwrap();
        });

    suite
        .query_unbonding(
            creator.clone().to_string(),
            "ampWHALE".to_string(),
            None,
            None,
            |res| {
                let unbonding_response = res.unwrap().1;
                assert_eq!(
                    unbonding_response,
                    UnbondingResponse {
                        total_amount: Uint128::new(1_000),
                        unbonding_requests: vec![
                            Bond {
                                id: 2,
                                asset: coin(100, "ampWHALE"),
                                created_at_epoch: 3,
                                unbonded_at: Some(1572056619),
                                last_updated: 3,
                                weight: Default::default(),
                                receiver: creator.clone()
                            },
                            Bond {
                                id: 3,
                                asset: coin(200, "ampWHALE"),
                                created_at_epoch: 3,
                                unbonded_at: Some(1572057619),
                                last_updated: 3,
                                weight: Default::default(),
                                receiver: creator.clone()
                            },
                            Bond {
                                id: 4,
                                asset: coin(300, "ampWHALE"),
                                created_at_epoch: 3,
                                unbonded_at: Some(1572058619),
                                last_updated: 3,
                                weight: Default::default(),
                                receiver: creator.clone()
                            },
                            Bond {
                                id: 5,
                                asset: coin(400, "ampWHALE"),
                                created_at_epoch: 3,
                                unbonded_at: Some(1572059619),
                                last_updated: 3,
                                weight: Default::default(),
                                receiver: creator.clone()
                            },
                        ],
                    }
                );
            },
        )
        .query_unbonding(
            creator.clone().to_string(),
            "ampWHALE".to_string(),
            None,
            Some(2),
            |res| {
                let unbonding_response = res.unwrap().1;
                assert_eq!(
                    unbonding_response,
                    UnbondingResponse {
                        total_amount: Uint128::new(300),
                        unbonding_requests: vec![
                            Bond {
                                id: 2,
                                asset: coin(100, "ampWHALE"),
                                created_at_epoch: 3,
                                unbonded_at: Some(1572056619),
                                last_updated: 3,
                                weight: Default::default(),
                                receiver: creator.clone(),
                            },
                            Bond {
                                id: 3,
                                asset: coin(200, "ampWHALE"),
                                created_at_epoch: 3,
                                unbonded_at: Some(1572057619),
                                last_updated: 3,
                                weight: Default::default(),
                                receiver: creator.clone(),
                            },
                        ],
                    }
                );
            },
        );

    suite.query_unbonding(
        creator.clone().to_string(),
        "ampWHALE".to_string(),
        Some(3),
        Some(2),
        |res| {
            let unbonding_response = res.unwrap().1;
            assert_eq!(
                unbonding_response,
                UnbondingResponse {
                    total_amount: Uint128::new(700),
                    unbonding_requests: vec![
                        Bond {
                            id: 4,
                            asset: coin(300, "ampWHALE"),
                            created_at_epoch: 3,
                            unbonded_at: Some(1572058619),
                            last_updated: 3,
                            weight: Default::default(),
                            receiver: creator.clone()
                        },
                        Bond {
                            id: 5,
                            asset: coin(400, "ampWHALE"),
                            created_at_epoch: 3,
                            unbonded_at: Some(1572059619),
                            last_updated: 3,
                            weight: Default::default(),
                            receiver: creator.clone()
                        },
                    ],
                }
            );
        },
    );

    // epoch 4
    suite.add_one_day().create_new_epoch();

    suite
        .query_global_index(Some(4u64), |result| {
            let global_index = result.unwrap().1;
            assert_eq!(
                global_index,
                GlobalIndex {
                    epoch_id: 4,
                    bonded_amount: Uint128::zero(),
                    bonded_assets: vec![],
                    last_updated: 4,
                    last_weight: Uint128::zero(),
                }
            );
        })
        .query_global_index(Some(5u64), |result| {
            let global_index = result.unwrap().1;
            assert_eq!(global_index, GlobalIndex::default());
        })
        .query_global_index(Some(100u64), |result| {
            let global_index = result.unwrap().1;
            assert_eq!(global_index, GlobalIndex::default());
        });
}
