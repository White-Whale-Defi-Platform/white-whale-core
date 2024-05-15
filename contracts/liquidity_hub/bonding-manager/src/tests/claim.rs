use cosmwasm_std::coin;
use cosmwasm_std::{coins, Coin, Decimal, Timestamp, Uint128};
use std::cell::RefCell;

use white_whale_std::bonding_manager::{GlobalIndex, RewardBucket};
use white_whale_std::fee::{Fee, PoolFee};
use white_whale_std::pool_network::asset::MINIMUM_LIQUIDITY_AMOUNT;

use crate::tests::suite::TestingSuite;
use crate::ContractError;

#[test]
fn test_claim_successfully() {
    let mut suite = TestingSuite::default();
    let creator = suite.senders[0].clone();
    let another_sender = suite.senders[1].clone();
    let yet_another_sender = suite.senders[2].clone();

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

    suite
        .instantiate_default()
        .fast_forward(259_200)
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
        // epoch 2
        .create_new_epoch()
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

    suite
        .query_claimable_reward_buckets(None, |res| {
            let (_, claimable_reward_buckets) = res.unwrap();
            assert_eq!(claimable_reward_buckets.len(), 1usize);

            assert_eq!(
                claimable_reward_buckets[0],
                RewardBucket {
                    id: 2,
                    epoch_start_time: Timestamp::from_nanos(1571970219879305533),
                    total: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(1_009),
                    }],
                    available: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(1_009),
                    }],
                    claimed: vec![],
                    global_index: GlobalIndex {
                        epoch_id: 2,
                        bonded_amount: Default::default(),
                        bonded_assets: vec![],
                        last_updated: 2,
                        last_weight: Default::default(),
                    },
                }
            );
        })
        // epoch 3
        .create_new_epoch()
        .query_claimable_reward_buckets(None, |res| {
            let (_, claimable_reward_buckets) = res.unwrap();
            assert_eq!(claimable_reward_buckets.len(), 2usize);

            assert_eq!(
                claimable_reward_buckets,
                vec![
                    RewardBucket {
                        id: 3,
                        epoch_start_time: Timestamp::from_nanos(1572056619879305533),
                        total: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(19),
                        }],
                        available: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(19),
                        }],
                        claimed: vec![],
                        global_index: GlobalIndex {
                            epoch_id: 3,
                            bonded_amount: Default::default(),
                            bonded_assets: vec![],
                            last_updated: 3,
                            last_weight: Default::default(),
                        },
                    },
                    RewardBucket {
                        id: 2,
                        epoch_start_time: Timestamp::from_nanos(1571970219879305533),
                        total: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(1_009),
                        }],
                        available: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(1_009),
                        }],
                        claimed: vec![],
                        global_index: GlobalIndex {
                            epoch_id: 2,
                            bonded_amount: Default::default(),
                            bonded_assets: vec![],
                            last_updated: 2,
                            last_weight: Default::default(),
                        },
                    },
                ]
            );
        });

    // we bond tokens with the creator
    suite
        .bond(
            creator.clone(),
            Coin {
                denom: "ampWHALE".to_string(),
                amount: Uint128::new(1_000u128),
            },
            &coins(1_000u128, "ampWHALE"),
            |res| {
                res.unwrap();
            },
        )
        .query_rewards(creator.clone().to_string(), |res| {
            let (_, rewards) = res.unwrap();
            // empty as the user just bonded
            assert!(rewards.rewards.is_empty());
        });

    // make some swaps to collect fees for the next epoch
    suite
        .swap(
            creator.clone(),
            coin(20_000u128, "uusdc"),
            "uwhale".to_string(),
            None,
            None,
            None,
            "whale-uusdc".to_string(),
            vec![Coin {
                denom: "uusdc".to_string(),
                amount: Uint128::from(20_000u128),
            }],
            |result| {
                result.unwrap();
            },
        )
        .swap(
            creator.clone(),
            coin(20_000u128, "uwhale"),
            "uusdc".to_string(),
            None,
            None,
            None,
            "whale-uusdc".to_string(),
            vec![Coin {
                denom: "uwhale".to_string(),
                amount: Uint128::from(20_000u128),
            }],
            |result| {
                result.unwrap();
            },
        );

    suite
        .add_one_day()
        // epoch 4
        .create_new_epoch()
        .query_claimable_reward_buckets(None, |res| {
            let claimable_reward_buckets = res.unwrap().1;

            assert_eq!(
                claimable_reward_buckets,
                vec![
                    RewardBucket {
                        id: 4,
                        epoch_start_time: Timestamp::from_nanos(1572143019879305533),
                        total: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(199),
                        }],
                        available: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(199),
                        }],
                        claimed: vec![],
                        global_index: GlobalIndex {
                            epoch_id: 4,
                            bonded_amount: Uint128::new(1_000),
                            bonded_assets: vec![Coin {
                                denom: "ampWHALE".to_string(),
                                amount: Uint128::new(1_000),
                            }],
                            last_updated: 3,
                            last_weight: Uint128::new(1_000),
                        },
                    },
                    RewardBucket {
                        id: 3,
                        epoch_start_time: Timestamp::from_nanos(1572056619879305533),
                        total: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(19),
                        }],
                        available: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(19),
                        }],
                        claimed: vec![],
                        global_index: GlobalIndex {
                            epoch_id: 3,
                            bonded_amount: Default::default(),
                            bonded_assets: vec![],
                            last_updated: 3,
                            last_weight: Default::default(),
                        },
                    },
                    RewardBucket {
                        id: 2,
                        epoch_start_time: Timestamp::from_nanos(1571970219879305533),
                        total: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(1_009),
                        }],
                        available: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(1_009),
                        }],
                        claimed: vec![],
                        global_index: GlobalIndex {
                            epoch_id: 2,
                            bonded_amount: Default::default(),
                            bonded_assets: vec![],
                            last_updated: 2,
                            last_weight: Default::default(),
                        },
                    },
                ]
            );
        });

    // we bond more tokens with another user

    suite
        .fast_forward(20_000)
        .bond(
            another_sender.clone(),
            Coin {
                denom: "bWHALE".to_string(),
                amount: Uint128::new(700u128),
            },
            &coins(700u128, "bWHALE"),
            |res| {
                res.unwrap();
            },
        )
        .query_rewards(another_sender.clone().to_string(), |res| {
            let (_, rewards) = res.unwrap();
            // empty as the user just bonded
            assert!(rewards.rewards.is_empty());
        });

    // let's make some swaps to fill the next buckets and then compare the users' rewards
    suite
        .swap(
            creator.clone(),
            coin(100_000u128, "uusdc"),
            "uwhale".to_string(),
            None,
            None,
            None,
            "whale-uusdc".to_string(),
            vec![Coin {
                denom: "uusdc".to_string(),
                amount: Uint128::from(100_000u128),
            }],
            |result| {
                result.unwrap();
            },
        )
        .add_one_day()
        // epoch 5
        .create_new_epoch();

    suite
        .query_claimable_reward_buckets(None, |res| {
            let claimable_reward_buckets = res.unwrap().1;

            assert_eq!(
                claimable_reward_buckets,
                vec![
                    RewardBucket {
                        id: 5,
                        epoch_start_time: Timestamp::from_nanos(1572229419879305533),
                        total: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(999),
                        }],
                        available: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(999),
                        }],
                        claimed: vec![],
                        global_index: GlobalIndex {
                            epoch_id: 5,
                            bonded_amount: Uint128::new(1_700u128),
                            bonded_assets: vec![
                                Coin {
                                    denom: "ampWHALE".to_string(),
                                    amount: Uint128::new(1_000),
                                },
                                Coin {
                                    denom: "bWHALE".to_string(),
                                    amount: Uint128::new(700),
                                },
                            ],
                            last_updated: 4,
                            last_weight: Uint128::new(2_700),
                        },
                    },
                    RewardBucket {
                        id: 4,
                        epoch_start_time: Timestamp::from_nanos(1572143019879305533),
                        total: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(199),
                        }],
                        available: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(199),
                        }],
                        claimed: vec![],
                        global_index: GlobalIndex {
                            epoch_id: 4,
                            bonded_amount: Uint128::new(1_000),
                            bonded_assets: vec![Coin {
                                denom: "ampWHALE".to_string(),
                                amount: Uint128::new(1_000),
                            }],
                            last_updated: 3,
                            last_weight: Uint128::new(1_000),
                        },
                    },
                    RewardBucket {
                        id: 3,
                        epoch_start_time: Timestamp::from_nanos(1572056619879305533),
                        total: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(19),
                        }],
                        available: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(19),
                        }],
                        claimed: vec![],
                        global_index: GlobalIndex {
                            epoch_id: 3,
                            bonded_amount: Default::default(),
                            bonded_assets: vec![],
                            last_updated: 3,
                            last_weight: Default::default(),
                        },
                    },
                    RewardBucket {
                        id: 2,
                        epoch_start_time: Timestamp::from_nanos(1571970219879305533),
                        total: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(1_009),
                        }],
                        available: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(1_009),
                        }],
                        claimed: vec![],
                        global_index: GlobalIndex {
                            epoch_id: 2,
                            bonded_amount: Default::default(),
                            bonded_assets: vec![],
                            last_updated: 2,
                            last_weight: Default::default(),
                        },
                    },
                ]
            );
        })
        .query_rewards(creator.clone().to_string(), |res| {
            let (_, rewards) = res.unwrap();
            assert_eq!(rewards.rewards.len(), 1);
            assert_eq!(rewards.rewards[0].amount, Uint128::new(880u128));
        })
        .query_rewards(another_sender.clone().to_string(), |res| {
            let (_, rewards) = res.unwrap();
            assert_eq!(rewards.rewards.len(), 1);
            assert_eq!(rewards.rewards[0].amount, Uint128::new(317u128));
        });

    suite
        .bond(
            another_sender.clone(),
            Coin {
                denom: "bWHALE".to_string(),
                amount: Uint128::new(1_000u128),
            },
            &coins(1_000u128, "bWHALE"),
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::UnclaimedRewards { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::UnclaimedRewards"),
                }
            },
        )
        .bond(
            yet_another_sender.clone(),
            Coin {
                denom: "bWHALE".to_string(),
                amount: Uint128::new(5_000u128),
            },
            &coins(5_000u128, "bWHALE"),
            |result| {
                result.unwrap();
            },
        );

    suite
        .swap(
            another_sender.clone(),
            coin(80_000u128, "uusdc"),
            "uwhale".to_string(),
            None,
            None,
            None,
            "whale-uusdc".to_string(),
            vec![Coin {
                denom: "uusdc".to_string(),
                amount: Uint128::from(80_000u128),
            }],
            |result| {
                result.unwrap();
            },
        )
        .add_one_day()
        // epoch 6
        .create_new_epoch();

    suite
        .query_claimable_reward_buckets(None, |res| {
            let claimable_reward_buckets = res.unwrap().1;

            assert_eq!(
                claimable_reward_buckets,
                vec![
                    RewardBucket {
                        id: 6,
                        epoch_start_time: Timestamp::from_nanos(1572315819879305533),
                        total: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(799),
                        }],
                        available: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(799),
                        }],
                        claimed: vec![],
                        global_index: GlobalIndex {
                            epoch_id: 6,
                            bonded_amount: Uint128::new(1_000 + 5_000 + 700),
                            bonded_assets: vec![
                                Coin {
                                    denom: "ampWHALE".to_string(),
                                    amount: Uint128::new(1_000),
                                },
                                Coin {
                                    denom: "bWHALE".to_string(),
                                    amount: Uint128::new(5_000 + 700),
                                },
                            ],
                            last_updated: 5,
                            last_weight: Uint128::new(9_400),
                        },
                    },
                    RewardBucket {
                        id: 5,
                        epoch_start_time: Timestamp::from_nanos(1572229419879305533),
                        total: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(999),
                        }],
                        available: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(999),
                        }],
                        claimed: vec![],
                        global_index: GlobalIndex {
                            epoch_id: 5,
                            bonded_amount: Uint128::new(1_000 + 700),
                            bonded_assets: vec![
                                Coin {
                                    denom: "ampWHALE".to_string(),
                                    amount: Uint128::new(1_000),
                                },
                                Coin {
                                    denom: "bWHALE".to_string(),
                                    amount: Uint128::new(700),
                                },
                            ],
                            last_updated: 4,
                            last_weight: Uint128::new(2_700),
                        },
                    },
                    RewardBucket {
                        id: 4,
                        epoch_start_time: Timestamp::from_nanos(1572143019879305533),
                        total: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(199),
                        }],
                        available: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(199),
                        }],
                        claimed: vec![],
                        global_index: GlobalIndex {
                            epoch_id: 4,
                            bonded_amount: Uint128::new(1_000),
                            bonded_assets: vec![Coin {
                                denom: "ampWHALE".to_string(),
                                amount: Uint128::new(1_000),
                            }],
                            last_updated: 3,
                            last_weight: Uint128::new(1_000),
                        },
                    },
                    RewardBucket {
                        id: 3,
                        epoch_start_time: Timestamp::from_nanos(1572056619879305533),
                        total: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(19),
                        }],
                        available: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(19),
                        }],
                        claimed: vec![],
                        global_index: GlobalIndex {
                            epoch_id: 3,
                            bonded_amount: Default::default(),
                            bonded_assets: vec![],
                            last_updated: 3,
                            last_weight: Default::default(),
                        },
                    },
                    RewardBucket {
                        id: 2,
                        epoch_start_time: Timestamp::from_nanos(1571970219879305533),
                        total: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(1_009),
                        }],
                        available: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(1_009),
                        }],
                        claimed: vec![],
                        global_index: GlobalIndex {
                            epoch_id: 2,
                            bonded_amount: Default::default(),
                            bonded_assets: vec![],
                            last_updated: 2,
                            last_weight: Default::default(),
                        },
                    },
                ]
            );
        })
        .query_rewards(creator.clone().to_string(), |res| {
            let (_, rewards) = res.unwrap();
            assert_eq!(rewards.rewards.len(), 1);
            assert_eq!(rewards.rewards[0].amount, Uint128::new(1078u128));
        })
        .query_rewards(another_sender.clone().to_string(), |res| {
            let (_, rewards) = res.unwrap();
            assert_eq!(rewards.rewards.len(), 1);
            assert_eq!(rewards.rewards[0].amount, Uint128::new(421u128));
        })
        .query_rewards(yet_another_sender.clone().to_string(), |res| {
            let (_, rewards) = res.unwrap();
            assert_eq!(rewards.rewards.len(), 1);
            assert_eq!(rewards.rewards[0].amount, Uint128::new(496u128));
        });

    // let's claim now

    let creator_balance = RefCell::new(Uint128::zero());
    let another_sender_balance = RefCell::new(Uint128::zero());
    let yet_another_sender_balance = RefCell::new(Uint128::zero());

    suite
        .query_balance("uwhale".to_string(), creator.clone(), |balance| {
            *creator_balance.borrow_mut() = balance;
        })
        .query_balance("uwhale".to_string(), another_sender.clone(), |balance| {
            *another_sender_balance.borrow_mut() = balance;
        })
        .query_balance(
            "uwhale".to_string(),
            yet_another_sender.clone(),
            |balance| {
                *yet_another_sender_balance.borrow_mut() = balance;
            },
        );

    suite
        .bond(
            creator.clone(),
            Coin {
                denom: "bWHALE".to_string(),
                amount: Uint128::new(1_000u128),
            },
            &coins(1_000u128, "bWHALE"),
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::UnclaimedRewards { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::UnclaimedRewards"),
                }
            },
        )
        .claim(creator.clone(), |result| {
            result.unwrap();
        })
        .bond(
            creator.clone(),
            Coin {
                denom: "bWHALE".to_string(),
                amount: Uint128::new(1_000u128),
            },
            &coins(1_000u128, "bWHALE"),
            |result| {
                result.unwrap();
            },
        )
        .claim(creator.clone(), |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();

            match err {
                ContractError::NothingToClaim { .. } => {}
                _ => panic!("Wrong error type, should return ContractError::NothingToClaim"),
            }
        })
        .claim(another_sender.clone(), |result| {
            result.unwrap();
        })
        .claim(yet_another_sender.clone(), |result| {
            result.unwrap();
        });

    suite
        .query_balance("uwhale".to_string(), creator.clone(), |balance| {
            assert_eq!(
                creator_balance.clone().into_inner() + Uint128::new(1078u128),
                balance
            );
        })
        .query_balance("uwhale".to_string(), another_sender.clone(), |balance| {
            assert_eq!(
                another_sender_balance.clone().into_inner() + Uint128::new(421u128),
                balance
            );
        })
        .query_balance(
            "uwhale".to_string(),
            yet_another_sender.clone(),
            |balance| {
                assert_eq!(
                    yet_another_sender_balance.clone().into_inner() + Uint128::new(496u128),
                    balance
                );
            },
        );

    suite.query_claimable_reward_buckets(None, |res| {
        let claimable_reward_buckets = res.unwrap().1;

        assert_eq!(
            claimable_reward_buckets,
            vec![
                RewardBucket {
                    id: 6,
                    epoch_start_time: Timestamp::from_nanos(1572315819879305533),
                    total: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(799),
                    }],
                    available: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(1),
                    }],
                    claimed: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(798),
                    }],
                    global_index: GlobalIndex {
                        epoch_id: 6,
                        bonded_amount: Uint128::new(1_000 + 5_000 + 700),
                        bonded_assets: vec![
                            Coin {
                                denom: "ampWHALE".to_string(),
                                amount: Uint128::new(1_000),
                            },
                            Coin {
                                denom: "bWHALE".to_string(),
                                amount: Uint128::new(5_000 + 700),
                            },
                        ],
                        last_updated: 5,
                        last_weight: Uint128::new(9_400),
                    },
                },
                RewardBucket {
                    id: 5,
                    epoch_start_time: Timestamp::from_nanos(1572229419879305533),
                    total: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(999),
                    }],
                    available: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(1),
                    }],
                    claimed: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(998),
                    }],
                    global_index: GlobalIndex {
                        epoch_id: 5,
                        bonded_amount: Uint128::new(1_000 + 700),
                        bonded_assets: vec![
                            Coin {
                                denom: "ampWHALE".to_string(),
                                amount: Uint128::new(1_000),
                            },
                            Coin {
                                denom: "bWHALE".to_string(),
                                amount: Uint128::new(700),
                            },
                        ],
                        last_updated: 4,
                        last_weight: Uint128::new(2_700),
                    },
                },
                RewardBucket {
                    id: 3,
                    epoch_start_time: Timestamp::from_nanos(1572056619879305533),
                    total: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(19),
                    }],
                    available: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(19),
                    }],
                    claimed: vec![],
                    global_index: GlobalIndex {
                        epoch_id: 3,
                        bonded_amount: Default::default(),
                        bonded_assets: vec![],
                        last_updated: 3,
                        last_weight: Default::default(),
                    },
                },
                RewardBucket {
                    id: 2,
                    epoch_start_time: Timestamp::from_nanos(1571970219879305533),
                    total: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(1_009),
                    }],
                    available: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(1_009),
                    }],
                    claimed: vec![],
                    global_index: GlobalIndex {
                        epoch_id: 2,
                        bonded_amount: Default::default(),
                        bonded_assets: vec![],
                        last_updated: 2,
                        last_weight: Default::default(),
                    },
                },
            ]
        );
    });
}

#[test]
fn test_rewards_forwarding() {
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

    suite
        .instantiate_default()
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
        );

    for i in 1..=20 {
        suite
            .swap(
                creator.clone(),
                coin(i * 1_000u128, "uusdc"),
                "uwhale".to_string(),
                None,
                None,
                None,
                "whale-uusdc".to_string(),
                vec![Coin {
                    denom: "uusdc".to_string(),
                    amount: Uint128::from(i * 1_000u128),
                }],
                |result| {
                    result.unwrap();
                },
            )
            .swap(
                creator.clone(),
                coin(i * 1_000u128, "uwhale"),
                "uusdc".to_string(),
                None,
                None,
                None,
                "whale-uusdc".to_string(),
                vec![Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::from(i * 1_000u128),
                }],
                |result| {
                    result.unwrap();
                },
            );

        suite.add_one_day().create_new_epoch();
    }

    suite.query_claimable_reward_buckets(None, |res| {
        let (_, claimable_reward_buckets) = res.unwrap();
        assert_eq!(claimable_reward_buckets.len(), 20usize);

        let first_bucket = claimable_reward_buckets.last().unwrap().clone();
        assert_eq!(
            first_bucket,
            RewardBucket {
                id: 1,
                epoch_start_time: Timestamp::from_nanos(1571883819879305533),
                total: vec![Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::new(1_009),
                }],
                available: vec![Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::new(1_009),
                }],
                claimed: vec![],
                global_index: GlobalIndex {
                    epoch_id: 1,
                    bonded_amount: Default::default(),
                    bonded_assets: vec![],
                    last_updated: 1,
                    last_weight: Default::default(),
                },
            }
        );

        assert_eq!(claimable_reward_buckets[0].id, 20u64);
        assert_eq!(claimable_reward_buckets[18].id, 2u64);
        assert_eq!(
            claimable_reward_buckets[18].available[0].amount,
            Uint128::new(19)
        );
    });

    // create two more epochs (without swapping, so they won't have any rewards), so the first bucket is forwarded to the one with id 22
    for i in 1..=2 {
        suite.add_one_day().create_new_epoch();
    }

    suite.query_claimable_reward_buckets(None, |res| {
        let (_, claimable_reward_buckets) = res.unwrap();

        assert_eq!(
            claimable_reward_buckets[0],
            RewardBucket {
                id: 22,
                epoch_start_time: Timestamp::from_nanos(1573698219879305533),
                total: vec![Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::new(1_009), // 1_009 forwarded from bucket 1
                }],
                available: vec![Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::new(1_009), // 1_009 forwarded from bucket 1
                }],
                claimed: vec![],
                global_index: GlobalIndex {
                    epoch_id: 22,
                    bonded_amount: Default::default(),
                    bonded_assets: vec![],
                    last_updated: 22,
                    last_weight: Default::default(),
                },
            }
        );

        assert_eq!(
            claimable_reward_buckets[19],
            RewardBucket {
                id: 2,
                epoch_start_time: Timestamp::from_nanos(1571970219879305533),
                total: vec![Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::new(19),
                }],
                available: vec![Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::new(19),
                }],
                claimed: vec![],
                global_index: GlobalIndex {
                    epoch_id: 2,
                    bonded_amount: Default::default(),
                    bonded_assets: vec![],
                    last_updated: 2,
                    last_weight: Default::default(),
                },
            }
        );
    });
}
