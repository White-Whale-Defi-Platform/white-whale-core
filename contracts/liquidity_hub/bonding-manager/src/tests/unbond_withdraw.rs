use std::cell::RefCell;

use cosmwasm_std::{coin, coins, Coin, Decimal, Timestamp, Uint128};

use white_whale_std::bonding_manager::{
    Bond, BondedResponse, GlobalIndex, RewardBucket, UnbondingResponse,
};
use white_whale_std::fee::{Fee, PoolFee};
use white_whale_std::pool_network::asset::MINIMUM_LIQUIDITY_AMOUNT;

use crate::tests::suite::TestingSuite;
use crate::ContractError;

#[test]
fn test_unbonding_withdraw() {
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
        .bond(creator.clone(), &coins(1_000u128, "ampWHALE"), |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();
            // the system has not been initialized
            match err {
                ContractError::Unauthorized { .. } => {}
                _ => panic!("Wrong error type, should return ContractError::Unauthorized"),
            }
        });

    suite
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

    // epoch 3
    suite.create_new_epoch();

    // we bond tokens with the creator
    suite
        .bond(creator.clone(), &coins(1_000u128, "ampWHALE"), |res| {
            res.unwrap();
        })
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
        .create_new_epoch();

    // we bond more tokens with another user

    suite
        .fast_forward(20_000)
        .bond(another_sender.clone(), &coins(700u128, "bWHALE"), |res| {
            res.unwrap();
        })
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

    suite.bond(
        yet_another_sender.clone(),
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

    // let's unbond

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
        .unbond(creator.clone(), coin(1_000u128, "bWHALE"), |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();
            // can't unbond if there are rewards to claim
            match err {
                ContractError::UnclaimedRewards { .. } => {}
                _ => panic!("Wrong error type, should return ContractError::UnclaimedRewards"),
            }
        })
        .claim(creator.clone(), |result| {
            result.unwrap();
        })
        .unbond(creator.clone(), coin(1_000u128, "bWHALE"), |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();
            // can't unbond an asset the user never bonded
            match err {
                ContractError::NothingToUnbond { .. } => {}
                _ => panic!("Wrong error type, should return ContractError::NothingToUnbond"),
            }
        })
        .unbond(creator.clone(), coin(100_000u128, "ampWHALE"), |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();
            // trying to unbond more than bonded
            match err {
                ContractError::InsufficientBond { .. } => {}
                _ => panic!("Wrong error type, should return ContractError::InsufficientBond"),
            }
        })
        .unbond(creator.clone(), coin(0u128, "ampWHALE"), |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();
            // trying to unbond more than bonded
            match err {
                ContractError::InvalidUnbondingAmount { .. } => {}
                _ => {
                    panic!("Wrong error type, should return ContractError::InvalidUnbondingAmount")
                }
            }
        })
        .unbond(creator.clone(), coin(1_000u128, "ampWHALE"), |result| {
            // total unbond
            result.unwrap();
        })
        .query_bonded(Some(creator.clone().to_string()), |res| {
            let bonded_response = res.unwrap().1;
            assert_eq!(bonded_response.total_bonded, Uint128::zero());
            assert!(bonded_response.bonded_assets.is_empty());
        })
        .query_unbonding(
            creator.clone().to_string(),
            "bWHALE".to_string(),
            None,
            None,
            |res| {
                let unbonding_response = res.unwrap().1;
                assert!(unbonding_response.unbonding_requests.is_empty());
            },
        )
        .query_unbonding(
            creator.clone().to_string(),
            "ampWHALE".to_string(),
            None,
            None,
            |res| {
                let unbonding_response = res.unwrap().1;
                assert_eq!(unbonding_response.unbonding_requests.len(), 1);
                assert_eq!(
                    unbonding_response.unbonding_requests[0],
                    Bond {
                        id: 4,
                        asset: coin(1_000u128, "ampWHALE"),
                        created_at_epoch: 6,
                        unbonded_at: Some(1572335819),
                        last_updated: 6,
                        weight: Uint128::zero(),
                        receiver: creator.clone(),
                    }
                );
            },
        )
        .query_rewards(creator.clone().to_string(), |res| {
            let (_, rewards) = res.unwrap();
            assert!(rewards.rewards.is_empty());
        })
        .withdraw(creator.clone(), "bWHALE".to_string(), |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();
            // trying to withdraw something the user never unbonded
            match err {
                ContractError::NothingToWithdraw { .. } => {}
                _ => panic!("Wrong error type, should return ContractError::NothingToWithdraw"),
            }
        })
        .withdraw(creator.clone(), "ampWHALE".to_string(), |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();
            // trying to withdraw before the unbonding period passed
            match err {
                ContractError::NothingToWithdraw { .. } => {}
                _ => panic!("Wrong error type, should return ContractError::NothingToWithdraw"),
            }
        })
        .query_withdrawable(creator.clone().to_string(), "ampWHALE".to_string(), |res| {
            let withdrawable = res.unwrap().1;
            assert_eq!(withdrawable.withdrawable_amount, Uint128::zero());
        })
        .add_one_day()
        .create_new_epoch();

    let creator_balance = RefCell::new(Uint128::zero());

    suite
        .query_balance("ampWHALE".to_string(), creator.clone(), |balance| {
            *creator_balance.borrow_mut() = balance;
        })
        .query_withdrawable(creator.clone().to_string(), "ampWHALE".to_string(), |res| {
            let withdrawable = res.unwrap().1;
            assert_eq!(withdrawable.withdrawable_amount, Uint128::new(1_000));
        })
        .withdraw(creator.clone(), "ampWHALE".to_string(), |result| {
            result.unwrap();
        })
        .query_balance("ampWHALE".to_string(), creator.clone(), |balance| {
            assert_eq!(
                creator_balance.clone().into_inner() + Uint128::new(1000u128),
                balance
            );
        })
        .query_rewards(creator.clone().to_string(), |res| {
            let (_, rewards) = res.unwrap();
            assert!(rewards.rewards.is_empty());
        });

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
                        amount: Uint128::new(601),
                    }],
                    claimed: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(198),
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
                        amount: Uint128::new(318),
                    }],
                    claimed: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(681),
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
        .create_new_epoch();

    suite
        .unbond(another_sender.clone(), coin(700u128, "bWHALE"), |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();
            // can't unbond if there are rewards to claim
            match err {
                ContractError::UnclaimedRewards { .. } => {}
                _ => panic!("Wrong error type, should return ContractError::UnclaimedRewards"),
            }
        })
        .claim(another_sender.clone(), |result| {
            result.unwrap();
        })
        .unbond(another_sender.clone(), coin(300u128, "bWHALE"), |result| {
            // partial unbond
            result.unwrap();
        })
        .fast_forward(1)
        .unbond(another_sender.clone(), coin(200u128, "bWHALE"), |result| {
            // partial unbond
            result.unwrap();
        })
        .query_bonded(Some(another_sender.clone().to_string()), |res| {
            let bonded_response = res.unwrap().1;
            assert_eq!(
                bonded_response,
                BondedResponse {
                    total_bonded: Uint128::new(200),
                    bonded_assets: coins(200u128, "bWHALE"),
                }
            );
        })
        .query_unbonding(
            another_sender.clone().to_string(),
            "bWHALE".to_string(),
            None,
            None,
            |res| {
                let unbonding_response = res.unwrap().1;
                assert_eq!(
                    unbonding_response,
                    UnbondingResponse {
                        total_amount: Uint128::new(500),
                        unbonding_requests: vec![
                            Bond {
                                id: 5,
                                asset: coin(300u128, "bWHALE"),
                                created_at_epoch: 8,
                                unbonded_at: Some(1572508619),
                                last_updated: 8,
                                weight: Uint128::zero(),
                                receiver: another_sender.clone(),
                            },
                            Bond {
                                id: 6,
                                asset: coin(200u128, "bWHALE"),
                                created_at_epoch: 8,
                                unbonded_at: Some(1572508620),
                                last_updated: 8,
                                weight: Uint128::zero(),
                                receiver: another_sender.clone(),
                            }
                        ],
                    }
                );
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
        .create_new_epoch();

    suite.query_claimable_reward_buckets(None, |res| {
        let claimable_reward_buckets = res.unwrap().1;

        assert_eq!(
            claimable_reward_buckets,
            vec![
                RewardBucket {
                    id: 9,
                    epoch_start_time: Timestamp::from_nanos(1572575019879305533),
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
                        epoch_id: 9,
                        bonded_amount: Uint128::new(5_000 + 200),
                        bonded_assets: vec![Coin {
                            denom: "bWHALE".to_string(),
                            amount: Uint128::new(5_000 + 200),
                        },],
                        last_updated: 8,
                        last_weight: Uint128::new(21_001),
                    },
                },
                RewardBucket {
                    id: 8,
                    epoch_start_time: Timestamp::from_nanos(1572488619879305533),
                    total: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(799),
                    }],
                    available: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(681),
                    }],
                    claimed: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(118),
                    }],
                    global_index: GlobalIndex {
                        epoch_id: 8,
                        bonded_amount: Uint128::new(5_000 + 700),
                        bonded_assets: vec![Coin {
                            denom: "bWHALE".to_string(),
                            amount: Uint128::new(5_000 + 700),
                        },],
                        last_updated: 6,
                        last_weight: Uint128::new(12_100),
                    },
                },
                RewardBucket {
                    id: 6,
                    epoch_start_time: Timestamp::from_nanos(1572315819879305533),
                    total: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(799),
                    }],
                    available: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(497),
                    }],
                    claimed: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(302),
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

    suite
        .withdraw(another_sender.clone(), "bWHALE".to_string(), |result| {
            result.unwrap();
        })
        .query_unbonding(
            another_sender.clone().to_string(),
            "bWHALE".to_string(),
            None,
            None,
            |res| {
                let unbonding_response = res.unwrap().1;
                assert!(unbonding_response.unbonding_requests.is_empty());
            },
        )
        .query_bonded(Some(another_sender.clone().to_string()), |res| {
            let bonded_response = res.unwrap().1;
            assert_eq!(
                bonded_response,
                BondedResponse {
                    total_bonded: Uint128::new(200),
                    bonded_assets: coins(200u128, "bWHALE"),
                }
            );
        })
        .unbond(another_sender.clone(), coin(200u128, "bWHALE"), |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();
            // can't unbond if there are rewards to claim
            match err {
                ContractError::UnclaimedRewards { .. } => {}
                _ => panic!("Wrong error type, should return ContractError::UnclaimedRewards"),
            }
        })
        .claim(another_sender.clone(), |result| {
            result.unwrap();
        })
        .unbond(another_sender.clone(), coin(200u128, "bWHALE"), |result| {
            // total unbond
            result.unwrap();
        });

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
        .create_new_epoch();

    suite
        .withdraw(another_sender.clone(), "bWHALE".to_string(), |result| {
            result.unwrap();
        })
        .query_unbonding(
            another_sender.clone().to_string(),
            "bWHALE".to_string(),
            None,
            None,
            |res| {
                let unbonding_response = res.unwrap().1;
                assert!(unbonding_response.unbonding_requests.is_empty());
            },
        )
        .query_bonded(Some(another_sender.clone().to_string()), |res| {
            let bonded_response = res.unwrap().1;
            assert_eq!(bonded_response.total_bonded, Uint128::zero());
            assert!(bonded_response.bonded_assets.is_empty());
        });

    suite.query_claimable_reward_buckets(None, |res| {
        let claimable_reward_buckets = res.unwrap().1;

        assert_eq!(
            claimable_reward_buckets,
            vec![
                RewardBucket {
                    id: 10,
                    epoch_start_time: Timestamp::from_nanos(1572661419879305533),
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
                        epoch_id: 10,
                        bonded_amount: Uint128::new(5_000),
                        bonded_assets: vec![Coin {
                            denom: "bWHALE".to_string(),
                            amount: Uint128::new(5_000),
                        },],
                        last_updated: 9,
                        // now the yet_another_sender has 100% of the weight
                        last_weight: Uint128::new(25_000),
                    },
                },
                RewardBucket {
                    id: 9,
                    epoch_start_time: Timestamp::from_nanos(1572575019879305533),
                    total: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(799),
                    }],
                    available: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(763),
                    }],
                    claimed: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(36),
                    }],
                    global_index: GlobalIndex {
                        epoch_id: 9,
                        bonded_amount: Uint128::new(5_000 + 200),
                        bonded_assets: vec![Coin {
                            denom: "bWHALE".to_string(),
                            amount: Uint128::new(5_000 + 200),
                        },],
                        last_updated: 8,
                        last_weight: Uint128::new(21_001),
                    },
                },
                RewardBucket {
                    id: 8,
                    epoch_start_time: Timestamp::from_nanos(1572488619879305533),
                    total: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(799),
                    }],
                    available: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(681),
                    }],
                    claimed: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(118),
                    }],
                    global_index: GlobalIndex {
                        epoch_id: 8,
                        bonded_amount: Uint128::new(5_000 + 700),
                        bonded_assets: vec![Coin {
                            denom: "bWHALE".to_string(),
                            amount: Uint128::new(5_000 + 700),
                        },],
                        last_updated: 6,
                        last_weight: Uint128::new(12_100),
                    },
                },
                RewardBucket {
                    id: 6,
                    epoch_start_time: Timestamp::from_nanos(1572315819879305533),
                    total: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(799),
                    }],
                    available: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(497),
                    }],
                    claimed: vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(302),
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

    suite
        .claim(yet_another_sender.clone(), |result| {
            result.unwrap();
        })
        .query_claimable_reward_buckets(None, |res| {
            let claimable_reward_buckets = res.unwrap().1;
            // epoch 10 disappeared because it was totally claimed by yet_another_sender
            assert_eq!(
                claimable_reward_buckets,
                vec![
                    RewardBucket {
                        id: 9,
                        epoch_start_time: Timestamp::from_nanos(1572575019879305533),
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
                            epoch_id: 9,
                            bonded_amount: Uint128::new(5_000 + 200),
                            bonded_assets: vec![Coin {
                                denom: "bWHALE".to_string(),
                                amount: Uint128::new(5_000 + 200),
                            },],
                            last_updated: 8,
                            last_weight: Uint128::new(21_001),
                        },
                    },
                    RewardBucket {
                        id: 8,
                        epoch_start_time: Timestamp::from_nanos(1572488619879305533),
                        total: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(799),
                        }],
                        available: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(2),
                        }],
                        claimed: vec![Coin {
                            denom: "uwhale".to_string(),
                            amount: Uint128::new(797),
                        }],
                        global_index: GlobalIndex {
                            epoch_id: 8,
                            bonded_amount: Uint128::new(5_000 + 700),
                            bonded_assets: vec![Coin {
                                denom: "bWHALE".to_string(),
                                amount: Uint128::new(5_000 + 700),
                            },],
                            last_updated: 6,
                            last_weight: Uint128::new(12_100),
                        },
                    },
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
