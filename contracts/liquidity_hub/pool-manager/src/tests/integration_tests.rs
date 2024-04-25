use cosmwasm_std::{coin, Addr, Coin, Decimal, Uint128};

use white_whale_std::fee::Fee;
use white_whale_std::fee::PoolFee;
use white_whale_std::pool_network::asset::MINIMUM_LIQUIDITY_AMOUNT;

use crate::ContractError;

use super::suite::TestingSuite;

// Using our suite lets test create pair
// and add liquidity to it

#[test]
fn instantiate_normal() {
    let mut suite = TestingSuite::default_with_balances(vec![]);

    suite.instantiate(suite.senders[0].to_string(), suite.senders[1].to_string());
}

// add features `token_factory` so tests are compiled using the correct flag
#[test]
fn deposit_and_withdraw_sanity_check() {
    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000u128, "uwhale".to_string()),
        coin(1_000_000u128, "uluna".to_string()),
        coin(1_000u128, "uusd".to_string()),
    ]);
    let creator = suite.creator();
    let _other = suite.senders[1].clone();
    let _unauthorized = suite.senders[2].clone();

    // Asset denoms with uwhale and uluna
    let asset_denoms = vec!["uwhale".to_string(), "uluna".to_string()];

    // Default Pool fees white_whale_std::pool_network::pair::PoolFee
    #[cfg(not(feature = "osmosis"))]
    let pool_fees = PoolFee {
        protocol_fee: Fee {
            share: Decimal::zero(),
        },
        swap_fee: Fee {
            share: Decimal::zero(),
        },
        burn_fee: Fee {
            share: Decimal::zero(),
        },
        extra_fees: vec![],
    };

    #[cfg(feature = "osmosis")]
    let pool_fees = PoolFee {
        protocol_fee: Fee {
            share: Decimal::zero(),
        },
        swap_fee: Fee {
            share: Decimal::zero(),
        },
        burn_fee: Fee {
            share: Decimal::zero(),
        },
        osmosis_fee: Fee {
            share: Decimal::zero(),
        },
        extra_fees: vec![],
    };

    // Create a pair
    suite.instantiate_default().create_pair(
        creator.clone(),
        asset_denoms,
        vec![6u8, 6u8],
        pool_fees,
        white_whale_std::pool_network::asset::PairType::ConstantProduct,
        Some("whale-uluna".to_string()),
        vec![coin(1000, "uusd")],
        |result| {
            result.unwrap();
        },
    );

    let contract_addr = suite.pool_manager_addr.clone();
    let lp_denom = suite.get_lp_denom("whale-uluna".to_string());

    // Lets try to add liquidity
    suite
        .provide_liquidity(
            creator.clone(),
            "whale-uluna".to_string(),
            None,
            None,
            vec![
                Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::from(1_000_000u128),
                },
                Coin {
                    denom: "uluna".to_string(),
                    amount: Uint128::from(1_000_000u128),
                },
            ],
            |result| {
                // Ensure we got 999_000 in the response which is 1_000_000 less the initial liquidity amount
                assert!(result.unwrap().events.iter().any(|event| {
                    event.attributes.iter().any(|attr| {
                        attr.key == "share"
                            && attr.value
                                == (Uint128::from(1_000_000u128) - MINIMUM_LIQUIDITY_AMOUNT)
                                    .to_string()
                    })
                }));
            },
        )
        // creator should have 999_000 LP shares (1M - MINIMUM_LIQUIDITY_AMOUNT)
        .query_all_balances(creator.to_string(), |result| {
            let balances = result.unwrap();
            println!("{:?}", balances);
            assert!(balances.iter().any(|coin| {
                coin.denom == lp_denom && coin.amount == Uint128::from(999_000u128)
            }));
        })
        // contract should have 1_000 LP shares (MINIMUM_LIQUIDITY_AMOUNT)
        .query_all_balances(contract_addr.to_string(), |result| {
            let balances = result.unwrap();
            // check that balances has 999_000 factory/migaloo1wug8sewp6cedgkmrmvhl3lf3tulagm9hnvy8p0rppz9yjw0g4wtqvk723g/uwhale-uluna.vault.whale-uluna.uLP
            assert!(balances.iter().any(|coin| {
                coin.denom == lp_denom.clone() && coin.amount == MINIMUM_LIQUIDITY_AMOUNT
            }));
        });

    // Lets try to withdraw liquidity
    suite
        .withdraw_liquidity(
            creator.clone(),
            "whale-uluna".to_string(),
            vec![Coin {
                denom: lp_denom.clone(),
                amount: Uint128::from(999_000u128),
            }],
            |result| {
                // we're trading 999_000 shares for 1_000_000 of our liquidity
                assert!(result.unwrap().events.iter().any(|event| {
                    event.attributes.iter().any(|attr| {
                        attr.key == "withdrawn_share"
                            && attr.value
                                == (Uint128::from(1_000_000u128) - MINIMUM_LIQUIDITY_AMOUNT)
                                    .to_string()
                    })
                }));
            },
        )
        // creator should have 0 LP shares in the contract and 0 LP shares in their account balance
        .query_amount_of_lp_token("whale-uluna".to_string(), creator.to_string(), |result| {
            assert_eq!(result.unwrap(), Uint128::zero());
        })
        .query_balance(creator.to_string(), lp_denom, |result| {
            assert_eq!(result.unwrap().amount, Uint128::zero());
        })
        // creator should 999_000 uwhale and 999_000 uluna (1M - MINIMUM_LIQUIDITY_AMOUNT)
        .query_all_balances(creator.to_string(), |result| {
            let balances = result.unwrap();
            assert!(balances.iter().any(|coin| {
                coin.denom == *"uwhale"
                    && coin.amount == Uint128::from(1_000_000u128) - MINIMUM_LIQUIDITY_AMOUNT
            }));
            assert!(balances.iter().any(|coin| {
                coin.denom == *"uluna"
                    && coin.amount == Uint128::from(1_000_000u128) - MINIMUM_LIQUIDITY_AMOUNT
            }));
        });
}

mod pair_creation_failures {
    use super::*;

    // Insufficient fee to create pair; 90 instead of 100
    #[test]
    fn insufficient_pair_creation_fee() {
        let mut suite = TestingSuite::default_with_balances(vec![
            coin(1_000_000_001u128, "uwhale".to_string()),
            coin(1_000_000_000u128, "uluna".to_string()),
            coin(1_000_000_001u128, "uusd".to_string()),
        ]);
        let creator = suite.creator();
        let _other = suite.senders[1].clone();
        let _unauthorized = suite.senders[2].clone();
        // Asset infos with uwhale and cw20

        let asset_infos = vec!["uwhale".to_string()];

        // Default Pool fees white_whale_std::pool_network::pair::PoolFee
        #[cfg(not(feature = "osmosis"))]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::zero(),
            },
            swap_fee: Fee {
                share: Decimal::zero(),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
            extra_fees: vec![],
        };

        #[cfg(feature = "osmosis")]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::zero(),
            },
            swap_fee: Fee {
                share: Decimal::zero(),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
            osmosis_fee: Fee {
                share: Decimal::zero(),
            },
            extra_fees: vec![],
        };
        // Create a pair
        suite.instantiate_default().create_pair(
            creator.clone(),
            asset_infos,
            vec![6u8, 6u8],
            pool_fees,
            white_whale_std::pool_network::asset::PairType::ConstantProduct,
            None,
            vec![coin(90, "uusd")],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::InvalidPairCreationFee { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::Unauthorized"),
                }
            },
        );
    }

    #[test]
    fn cant_recreate_existing_pair() {
        let mut suite = TestingSuite::default_with_balances(vec![
            coin(1_000_000_001u128, "uwhale".to_string()),
            coin(1_000_000_000u128, "uluna".to_string()),
            coin(1_000_000_001u128, "uusd".to_string()),
        ]);
        let creator = suite.creator();
        let _other = suite.senders[1].clone();
        let _unauthorized = suite.senders[2].clone();
        // Asset infos with uwhale and cw20

        let asset_infos = vec!["uwhale".to_string()];

        // Default Pool fees white_whale_std::pool_network::pair::PoolFee
        #[cfg(not(feature = "osmosis"))]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::zero(),
            },
            swap_fee: Fee {
                share: Decimal::zero(),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
            extra_fees: vec![],
        };

        #[cfg(feature = "osmosis")]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::zero(),
            },
            swap_fee: Fee {
                share: Decimal::zero(),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
            osmosis_fee: Fee {
                share: Decimal::zero(),
            },
            extra_fees: vec![],
        };

        // Create a pair
        suite
            .instantiate_default()
            .create_pair(
                creator.clone(),
                asset_infos.clone(),
                vec![6u8, 6u8],
                pool_fees.clone(),
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                Some("mycoolpair".to_string()),
                vec![coin(1000, "uusd")],
                |result| {
                    result.unwrap();
                },
            )
            .create_pair(
                creator.clone(),
                asset_infos,
                vec![6u8, 6u8],
                pool_fees,
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                Some("mycoolpair".to_string()),
                vec![coin(1000, "uusd")],
                |result| {
                    let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                    match err {
                        ContractError::PairExists { .. } => {}
                        _ => panic!("Wrong error type, should return ContractError::PairExists"),
                    }
                },
            );
    }
}

mod router {
    use cosmwasm_std::Event;
    use white_whale_std::pool_manager::{SwapRoute, SwapRouteCreatorResponse};

    use super::*;

    #[test]
    fn basic_swap_operations_test() {
        let mut suite = TestingSuite::default_with_balances(vec![
            coin(1_000_000_000u128, "uwhale".to_string()),
            coin(1_000_000_000u128, "uluna".to_string()),
            coin(1_000_000_000u128, "uusd".to_string()),
        ]);
        let creator = suite.creator();
        let _other = suite.senders[1].clone();
        let _unauthorized = suite.senders[2].clone();
        // Asset infos with uwhale and uluna

        let first_pair = vec!["uwhale".to_string(), "uluna".to_string()];
        let second_pair = vec!["uluna".to_string(), "uusd".to_string()];

        #[cfg(not(feature = "osmosis"))]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::bps(50), // 0.5%
            },
            swap_fee: Fee {
                share: Decimal::bps(50), // 0.5%
            },
            burn_fee: Fee {
                share: Decimal::bps(50), // 0.5%
            },
            extra_fees: vec![],
        };
        #[cfg(feature = "osmosis")]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::bps(50),
            },
            swap_fee: Fee {
                share: Decimal::bps(50),
            },
            burn_fee: Fee {
                share: Decimal::bps(50),
            },
            osmosis_fee: Fee {
                share: Decimal::bps(50),
            },
            extra_fees: vec![],
        };

        // Create a pair
        suite
            .instantiate_default()
            .create_pair(
                creator.clone(),
                first_pair,
                vec![6u8, 6u8],
                pool_fees.clone(),
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                Some("whale-uluna".to_string()),
                vec![coin(1000, "uusd")],
                |result| {
                    result.unwrap();
                },
            )
            .create_pair(
                creator.clone(),
                second_pair,
                vec![6u8, 6u8],
                pool_fees,
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                Some("uluna-uusd".to_string()),
                vec![coin(1000, "uusd")],
                |result| {
                    result.unwrap();
                },
            );

        // Lets try to add liquidity
        suite.provide_liquidity(
            creator.clone(),
            "whale-uluna".to_string(),
            None,
            None,
            vec![
                Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::from(1000000u128),
                },
                Coin {
                    denom: "uluna".to_string(),
                    amount: Uint128::from(1000000u128),
                },
            ],
            |result| {
                // ensure we got 999,000 in the response (1m - initial liquidity amount)
                let result = result.unwrap();
                assert!(result.has_event(&Event::new("wasm").add_attribute("share", "999000")));
            },
        );

        // Lets try to add liquidity
        suite.provide_liquidity(
            creator.clone(),
            "uluna-uusd".to_string(),
            None,
            None,
            vec![
                Coin {
                    denom: "uluna".to_string(),
                    amount: Uint128::from(1000000u128),
                },
                Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(1000000u128),
                },
            ],
            |result| {
                // ensure we got 999,000 in the response (1m - initial liquidity amount)
                let result = result.unwrap();
                assert!(result.has_event(&Event::new("wasm").add_attribute("share", "999000")));
            },
        );

        // Prepare the swap operations, we want to go from WHALE -> UUSD
        // We will use the uluna-uusd pair as the intermediary pool

        let swap_operations = vec![
            white_whale_std::pool_manager::SwapOperation::WhaleSwap {
                token_in_denom: "uwhale".to_string(),
                token_out_denom: "uluna".to_string(),
                pool_identifier: "whale-uluna".to_string(),
            },
            white_whale_std::pool_manager::SwapOperation::WhaleSwap {
                token_in_denom: "uluna".to_string(),
                token_out_denom: "uusd".to_string(),
                pool_identifier: "uluna-uusd".to_string(),
            },
        ];

        // before swap uusd balance = 1_000_000_000
        // - 2*1_000 pair creation fee
        // - 1_000_000 liquidity provision
        // = 998_998_000
        let pre_swap_amount = 998_998_000;
        suite.query_balance(creator.to_string(), "uusd".to_string(), |amt| {
            assert_eq!(amt.unwrap().amount.u128(), pre_swap_amount);
        });

        suite.execute_swap_operations(
            creator.clone(),
            swap_operations,
            None,
            None,
            None,
            vec![coin(1000u128, "uwhale".to_string())],
            |result| {
                result.unwrap();
            },
        );

        // ensure that the whale got swapped to an appropriate amount of uusd
        // we swap 1000 whale for 974 uusd
        // with a fee of 4*6 = 24 uusd
        let post_swap_amount = pre_swap_amount + 974;
        suite.query_balance(creator.to_string(), "uusd".to_string(), |amt| {
            assert_eq!(amt.unwrap().amount.u128(), post_swap_amount);
        });

        // ensure that fees got sent to the appropriate place
        suite.query_balance(
            suite.bonding_manager_addr.to_string(),
            "uusd".to_string(),
            |amt| {
                assert_eq!(amt.unwrap().amount.u128(), 2000 + 4 * 2);
            },
        );
        suite.query_balance(
            suite.bonding_manager_addr.to_string(),
            "uwhale".to_string(),
            |amt| {
                assert_eq!(amt.unwrap().amount.u128(), 0);
            },
        );
        suite.query_balance(
            suite.bonding_manager_addr.to_string(),
            "uluna".to_string(),
            |amt| {
                assert_eq!(amt.unwrap().amount.u128(), 4 * 2);
            },
        );
    }

    #[test]
    fn rejects_empty_swaps() {
        let mut suite = TestingSuite::default_with_balances(vec![
            coin(1_000_000_001u128, "uwhale".to_string()),
            coin(1_000_000_000u128, "uluna".to_string()),
            coin(1_000_000_001u128, "uusd".to_string()),
        ]);
        let creator = suite.creator();
        let _other = suite.senders[1].clone();
        let _unauthorized = suite.senders[2].clone();
        // Asset infos with uwhale and uluna

        let first_pair = vec!["uwhale".to_string(), "uluna".to_string()];
        let second_pair = vec!["uluna".to_string(), "uusd".to_string()];

        // Default Pool fees white_whale_std::pool_network::pair::PoolFee
        // Protocol fee is 0.01% and swap fee is 0.02% and burn fee is 0%
        #[cfg(not(feature = "osmosis"))]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::from_ratio(1u128, 100_000u128),
            },
            swap_fee: Fee {
                share: Decimal::from_ratio(1u128, 100_000u128),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
            extra_fees: vec![],
        };

        #[cfg(feature = "osmosis")]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::from_ratio(1u128, 100_000u128),
            },
            swap_fee: Fee {
                share: Decimal::from_ratio(1u128, 100_000u128),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
            osmosis_fee: Fee {
                share: Decimal::from_ratio(1u128, 100_000u128),
            },
            extra_fees: vec![],
        };

        // Create a pair
        suite
            .instantiate_default()
            .create_pair(
                creator.clone(),
                first_pair,
                vec![6u8, 6u8],
                pool_fees.clone(),
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                Some("whale-uluna".to_string()),
                vec![coin(1000, "uusd")],
                |result| {
                    result.unwrap();
                },
            )
            .create_pair(
                creator.clone(),
                second_pair,
                vec![6u8, 6u8],
                pool_fees,
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                Some("uluna-uusd".to_string()),
                vec![coin(1000, "uusd")],
                |result| {
                    result.unwrap();
                },
            );

        // Lets try to add liquidity
        suite.provide_liquidity(
            creator.clone(),
            "whale-uluna".to_string(),
            None,
            None,
            vec![
                Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::from(1000000u128),
                },
                Coin {
                    denom: "uluna".to_string(),
                    amount: Uint128::from(1000000u128),
                },
            ],
            |result| {
                // ensure we got 999,000 in the response (1m - initial liquidity amount)
                let result = result.unwrap();
                assert!(result.has_event(&Event::new("wasm").add_attribute("share", "999000")));
            },
        );

        // Lets try to add liquidity
        suite.provide_liquidity(
            creator.clone(),
            "uluna-uusd".to_string(),
            None,
            None,
            vec![
                Coin {
                    denom: "uluna".to_string(),
                    amount: Uint128::from(1000000u128),
                },
                Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(1000000u128),
                },
            ],
            |result| {
                // ensure we got 999,000 in the response (1m - initial liquidity amount)
                let result = result.unwrap();
                assert!(result.has_event(&Event::new("wasm").add_attribute("share", "999000")));
            },
        );

        // attempt to perform a 0 swap operations
        let swap_operations = vec![];

        suite.execute_swap_operations(
            creator.clone(),
            swap_operations,
            None,
            None,
            None,
            vec![coin(1000u128, "uwhale".to_string())],
            |result| {
                assert_eq!(
                    result.unwrap_err().downcast_ref::<ContractError>(),
                    Some(&ContractError::NoSwapOperationsProvided {})
                )
            },
        );
    }

    #[test]
    fn rejects_non_consecutive_swaps() {
        let mut suite = TestingSuite::default_with_balances(vec![
            coin(1_000_000_001u128, "uwhale".to_string()),
            coin(1_000_000_000u128, "uluna".to_string()),
            coin(1_000_000_001u128, "uusd".to_string()),
        ]);
        let creator = suite.creator();
        let other = suite.senders[1].clone();
        let _unauthorized = suite.senders[2].clone();
        // Asset infos with uwhale and uluna

        let first_pair = vec!["uwhale".to_string(), "uluna".to_string()];
        let second_pair = vec!["uluna".to_string(), "uusd".to_string()];

        // Default Pool fees white_whale_std::pool_network::pair::PoolFee
        // Protocol fee is 0.01% and swap fee is 0.02% and burn fee is 0%
        #[cfg(not(feature = "osmosis"))]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::from_ratio(1u128, 100_000u128),
            },
            swap_fee: Fee {
                share: Decimal::from_ratio(1u128, 100_000u128),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
            extra_fees: vec![],
        };

        #[cfg(feature = "osmosis")]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::from_ratio(1u128, 100_000u128),
            },
            swap_fee: Fee {
                share: Decimal::from_ratio(1u128, 100_000u128),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
            osmosis_fee: Fee {
                share: Decimal::from_ratio(1u128, 100_000u128),
            },
            extra_fees: vec![],
        };

        // Create a pair
        suite
            .instantiate_default()
            .create_pair(
                creator.clone(),
                first_pair,
                vec![6u8, 6u8],
                pool_fees.clone(),
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                Some("whale-uluna".to_string()),
                vec![coin(1000, "uusd")],
                |result| {
                    result.unwrap();
                },
            )
            .create_pair(
                creator.clone(),
                second_pair,
                vec![6u8, 6u8],
                pool_fees,
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                Some("uluna-uusd".to_string()),
                vec![coin(1000, "uusd")],
                |result| {
                    result.unwrap();
                },
            );

        // Lets try to add liquidity
        suite.provide_liquidity(
            creator.clone(),
            "whale-uluna".to_string(),
            None,
            None,
            vec![
                Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::from(1000000u128),
                },
                Coin {
                    denom: "uluna".to_string(),
                    amount: Uint128::from(1000000u128),
                },
            ],
            |result| {
                // ensure we got 999,000 in the response (1m - initial liquidity amount)
                let result = result.unwrap();
                println!("{:?}", result);
                assert!(result.has_event(&Event::new("wasm").add_attribute("share", "999000")));
            },
        );

        // Lets try to add liquidity
        suite.provide_liquidity(
            creator.clone(),
            "uluna-uusd".to_string(),
            None,
            None,
            vec![
                Coin {
                    denom: "uluna".to_string(),
                    amount: Uint128::from(1000000u128),
                },
                Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(1000000u128),
                },
            ],
            |result| {
                // ensure we got 999,000 in the response (1m - initial liquidity amount)
                let result = result.unwrap();
                assert!(result.has_event(&Event::new("wasm").add_attribute("share", "999000")));
            },
        );

        // Prepare the swap operations, we want to go from WHALE -> UUSD
        // We will use the uluna-uusd pair as the intermediary pool

        let swap_operations = vec![
            white_whale_std::pool_manager::SwapOperation::WhaleSwap {
                token_in_denom: "uwhale".to_string(),
                token_out_denom: "uluna".to_string(),
                pool_identifier: "whale-uluna".to_string(),
            },
            white_whale_std::pool_manager::SwapOperation::WhaleSwap {
                token_in_denom: "uwhale".to_string(),
                token_out_denom: "uluna".to_string(),
                pool_identifier: "whale-uluna".to_string(),
            },
        ];

        suite.execute_swap_operations(
            other.clone(),
            swap_operations,
            None,
            None,
            None,
            vec![coin(1000u128, "uwhale".to_string())],
            |result| {
                assert_eq!(
                    result.unwrap_err().downcast_ref::<self::ContractError>(),
                    Some(&ContractError::NonConsecutiveSwapOperations {
                        previous_output: "uluna".to_string(),
                        next_input: "uwhale".to_string(),
                    })
                );
            },
        );
    }

    #[test]
    fn sends_to_correct_receiver() {
        let mut suite = TestingSuite::default_with_balances(vec![
            coin(1_000_000_000u128, "uwhale".to_string()),
            coin(1_000_000_000u128, "uluna".to_string()),
            coin(1_000_000_000u128, "uusd".to_string()),
        ]);
        let creator = suite.creator();
        let other = suite.senders[1].clone();
        let unauthorized = suite.senders[2].clone();
        // Asset infos with uwhale and uluna

        let first_pair = vec!["uwhale".to_string(), "uluna".to_string()];
        let second_pair = vec!["uluna".to_string(), "uusd".to_string()];

        // Default Pool fees white_whale_std::pool_network::pair::PoolFee
        // Protocol fee is 0.01% and swap fee is 0.02% and burn fee is 0%
        #[cfg(not(feature = "osmosis"))]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::from_ratio(1u128, 100_000u128),
            },
            swap_fee: Fee {
                share: Decimal::from_ratio(1u128, 100_000u128),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
            extra_fees: vec![],
        };

        #[cfg(feature = "osmosis")]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::from_ratio(1u128, 100_000u128),
            },
            swap_fee: Fee {
                share: Decimal::from_ratio(1u128, 100_000u128),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
            osmosis_fee: Fee {
                share: Decimal::from_ratio(1u128, 100_000u128),
            },
            extra_fees: vec![],
        };

        // Create a pair
        suite
            .instantiate_default()
            .create_pair(
                creator.clone(),
                first_pair,
                vec![6u8, 6u8],
                pool_fees.clone(),
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                Some("whale-uluna".to_string()),
                vec![coin(1000, "uusd")],
                |result| {
                    result.unwrap();
                },
            )
            .create_pair(
                creator.clone(),
                second_pair,
                vec![6u8, 6u8],
                pool_fees,
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                Some("uluna-uusd".to_string()),
                vec![coin(1000, "uusd")],
                |result| {
                    result.unwrap();
                },
            );

        // Lets try to add liquidity
        let liquidity_amount = 1_000_000u128;
        suite.provide_liquidity(
            creator.clone(),
            "whale-uluna".to_string(),
            None,
            None,
            vec![
                Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::from(liquidity_amount),
                },
                Coin {
                    denom: "uluna".to_string(),
                    amount: Uint128::from(liquidity_amount),
                },
            ],
            |result| {
                // ensure we got 999,000 in the response (1m - initial liquidity amount)
                let result = result.unwrap();
                assert!(result.has_event(&Event::new("wasm").add_attribute("share", "999000")));
            },
        );

        // Lets try to add liquidity
        suite.provide_liquidity(
            creator.clone(),
            "uluna-uusd".to_string(),
            None,
            None,
            vec![
                Coin {
                    denom: "uluna".to_string(),
                    amount: Uint128::from(liquidity_amount),
                },
                Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(liquidity_amount),
                },
            ],
            |result| {
                // ensure we got 999,000 in the response (1m - initial liquidity amount)
                let result = result.unwrap();
                assert!(result.has_event(&Event::new("wasm").add_attribute("share", "999000")));
            },
        );

        // Prepare the swap operations, we want to go from WHALE -> UUSD
        // We will use the uluna-uusd pair as the intermediary pool

        let swap_operations = vec![
            white_whale_std::pool_manager::SwapOperation::WhaleSwap {
                token_in_denom: "uwhale".to_string(),
                token_out_denom: "uluna".to_string(),
                pool_identifier: "whale-uluna".to_string(),
            },
            white_whale_std::pool_manager::SwapOperation::WhaleSwap {
                token_in_denom: "uluna".to_string(),
                token_out_denom: "uusd".to_string(),
                pool_identifier: "uluna-uusd".to_string(),
            },
        ];

        // before swap uusd balance = 1_000_000_000
        // before swap uwhale balance = 1_000_000_000
        // before swap uluna balance = 1_000_000_000
        let pre_swap_amount = 1_000_000_000;
        suite.query_balance(other.to_string(), "uusd".to_string(), |amt| {
            assert_eq!(amt.unwrap().amount.u128(), pre_swap_amount);
        });
        suite.query_balance(other.to_string(), "uwhale".to_string(), |amt| {
            assert_eq!(amt.unwrap().amount.u128(), pre_swap_amount);
        });
        suite.query_balance(other.to_string(), "uluna".to_string(), |amt| {
            assert_eq!(amt.unwrap().amount.u128(), pre_swap_amount);
        });
        // also check the same for unauthorized receiver
        suite.query_balance(other.to_string(), "uusd".to_string(), |amt| {
            assert_eq!(amt.unwrap().amount.u128(), pre_swap_amount);
        });
        suite.query_balance(other.to_string(), "uwhale".to_string(), |amt| {
            assert_eq!(amt.unwrap().amount.u128(), pre_swap_amount);
        });
        suite.query_balance(other.to_string(), "uluna".to_string(), |amt| {
            assert_eq!(amt.unwrap().amount.u128(), pre_swap_amount);
        });
        // also check for contract
        suite.query_balance(
            suite.pool_manager_addr.to_string(),
            "uusd".to_string(),
            |amt| {
                assert_eq!(amt.unwrap().amount.u128(), liquidity_amount);
            },
        );
        suite.query_balance(
            suite.pool_manager_addr.to_string(),
            "uwhale".to_string(),
            |amt| {
                assert_eq!(amt.unwrap().amount.u128(), liquidity_amount);
            },
        );
        suite.query_balance(
            suite.pool_manager_addr.to_string(),
            "uluna".to_string(),
            |amt| {
                assert_eq!(amt.unwrap().amount.u128(), 2 * liquidity_amount);
            },
        );

        // perform swaps
        suite.execute_swap_operations(
            other.clone(),
            swap_operations,
            None,
            Some(unauthorized.to_string()),
            None,
            vec![coin(1000u128, "uwhale".to_string())],
            |result| {
                result.unwrap();
            },
        );

        // ensure that the whale got swapped to an appropriate amount of uusd
        // we swap 1000 whale for 998 uusd
        let post_swap_amount = pre_swap_amount + 998;
        suite.query_balance(unauthorized.to_string(), "uusd".to_string(), |amt| {
            assert_eq!(amt.unwrap().amount.u128(), post_swap_amount);
        });
        // check that the balances of the contract are ok
        suite.query_balance(
            suite.pool_manager_addr.to_string(),
            "uusd".to_string(),
            |amt| {
                assert_eq!(amt.unwrap().amount.u128(), liquidity_amount - 998);
            },
        );
        suite.query_balance(
            suite.pool_manager_addr.to_string(),
            "uwhale".to_string(),
            |amt| {
                assert_eq!(amt.unwrap().amount.u128(), liquidity_amount + 1000);
            },
        );
        suite.query_balance(
            suite.pool_manager_addr.to_string(),
            "uluna".to_string(),
            |amt| {
                assert_eq!(amt.unwrap().amount.u128(), 2 * liquidity_amount);
            },
        );
    }

    #[test]
    fn checks_minimum_receive() {
        let mut suite = TestingSuite::default_with_balances(vec![
            coin(1_000_000_000u128, "uwhale".to_string()),
            coin(1_000_000_000u128, "uluna".to_string()),
            coin(1_000_000_000u128, "uusd".to_string()),
        ]);
        let creator = suite.creator();
        let _other = suite.senders[1].clone();
        let _unauthorized = suite.senders[2].clone();
        // Asset infos with uwhale and uluna

        let first_pair = vec!["uwhale".to_string(), "uluna".to_string()];
        let second_pair = vec!["uluna".to_string(), "uusd".to_string()];

        #[cfg(not(feature = "osmosis"))]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::bps(50), // 0.5%
            },
            swap_fee: Fee {
                share: Decimal::bps(50), // 0.5%
            },
            burn_fee: Fee {
                share: Decimal::bps(50), // 0.5%
            },
            extra_fees: vec![],
        };
        #[cfg(feature = "osmosis")]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::bps(50),
            },
            swap_fee: Fee {
                share: Decimal::bps(50),
            },
            burn_fee: Fee {
                share: Decimal::bps(50),
            },
            osmosis_fee: Fee {
                share: Decimal::bps(50),
            },
            extra_fees: vec![],
        };

        // Create a pair
        suite
            .instantiate_default()
            .create_pair(
                creator.clone(),
                first_pair,
                vec![6u8, 6u8],
                pool_fees.clone(),
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                Some("whale-uluna".to_string()),
                vec![coin(1000, "uusd")],
                |result| {
                    result.unwrap();
                },
            )
            .create_pair(
                creator.clone(),
                second_pair,
                vec![6u8, 6u8],
                pool_fees,
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                Some("uluna-uusd".to_string()),
                vec![coin(1000, "uusd")],
                |result| {
                    result.unwrap();
                },
            );

        // Lets try to add liquidity
        suite.provide_liquidity(
            creator.clone(),
            "whale-uluna".to_string(),
            None,
            None,
            vec![
                Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::from(1000000u128),
                },
                Coin {
                    denom: "uluna".to_string(),
                    amount: Uint128::from(1000000u128),
                },
            ],
            |result| {
                // ensure we got 999,000 in the response (1m - initial liquidity amount)
                let result = result.unwrap();
                assert!(result.has_event(&Event::new("wasm").add_attribute("share", "999000")));
            },
        );

        // Lets try to add liquidity
        suite.provide_liquidity(
            creator.clone(),
            "uluna-uusd".to_string(),
            None,
            None,
            vec![
                Coin {
                    denom: "uluna".to_string(),
                    amount: Uint128::from(1000000u128),
                },
                Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(1000000u128),
                },
            ],
            |result| {
                // ensure we got 999,000 in the response (1m - initial liquidity amount)
                let result = result.unwrap();
                assert!(result.has_event(&Event::new("wasm").add_attribute("share", "999000")));
            },
        );

        // Prepare the swap operations, we want to go from WHALE -> UUSD
        // We will use the uluna-uusd pair as the intermediary pool

        let swap_operations = vec![
            white_whale_std::pool_manager::SwapOperation::WhaleSwap {
                token_in_denom: "uwhale".to_string(),
                token_out_denom: "uluna".to_string(),
                pool_identifier: "whale-uluna".to_string(),
            },
            white_whale_std::pool_manager::SwapOperation::WhaleSwap {
                token_in_denom: "uluna".to_string(),
                token_out_denom: "uusd".to_string(),
                pool_identifier: "uluna-uusd".to_string(),
            },
        ];

        // before swap uusd balance = 1_000_000_000
        // - 2*1_000 pair creation fee
        // - 1_000_000 liquidity provision
        // = 998_998_000
        let pre_swap_amount = 998_998_000;
        suite.query_balance(creator.to_string(), "uusd".to_string(), |amt| {
            assert_eq!(amt.unwrap().amount.u128(), pre_swap_amount);
        });

        // require an output of 975 uusd
        suite.execute_swap_operations(
            creator.clone(),
            swap_operations,
            Some(Uint128::new(975)),
            None,
            None,
            vec![coin(1000u128, "uwhale".to_string())],
            |result| {
                assert_eq!(
                    result.unwrap_err().downcast_ref::<ContractError>(),
                    Some(&ContractError::MinimumReceiveAssertion {
                        minimum_receive: Uint128::new(975),
                        swap_amount: Uint128::new(974),
                    })
                )
            },
        );
    }

    #[test]
    fn add_swap_routes() {
        let mut suite = TestingSuite::default_with_balances(vec![
            coin(1_000_000_001u128, "uwhale".to_string()),
            coin(1_000_000_000u128, "uluna".to_string()),
            coin(1_000_000_001u128, "uusd".to_string()),
        ]);
        let creator = suite.creator();
        let _other = suite.senders[1].clone();
        let _unauthorized = suite.senders[2].clone();

        let first_pair = vec!["uwhale".to_string(), "uluna".to_string()];
        let second_pair = vec!["uluna".to_string(), "uusd".to_string()];

        #[cfg(not(feature = "osmosis"))]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::bps(50), // 0.5%
            },
            swap_fee: Fee {
                share: Decimal::bps(50), // 0.5%
            },
            burn_fee: Fee {
                share: Decimal::bps(50), // 0.5%
            },
            extra_fees: vec![],
        };
        #[cfg(feature = "osmosis")]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::bps(50),
            },
            swap_fee: Fee {
                share: Decimal::bps(50),
            },
            burn_fee: Fee {
                share: Decimal::bps(50),
            },
            osmosis_fee: Fee {
                share: Decimal::bps(50),
            },
            extra_fees: vec![],
        };

        // Create a pair
        suite
            .instantiate_default()
            .create_pair(
                creator.clone(),
                first_pair,
                vec![6u8, 6u8],
                pool_fees.clone(),
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                Some("whale-uluna".to_string()),
                vec![coin(1000, "uusd")],
                |result| {
                    result.unwrap();
                },
            )
            .create_pair(
                creator.clone(),
                second_pair,
                vec![6u8, 6u8],
                pool_fees,
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                Some("uluna-uusd".to_string()),
                vec![coin(1000, "uusd")],
                |result| {
                    result.unwrap();
                },
            )
            .provide_liquidity(
                creator.clone(),
                "whale-uluna".to_string(),
                vec![
                    Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::from(1000000u128),
                    },
                    Coin {
                        denom: "uluna".to_string(),
                        amount: Uint128::from(1000000u128),
                    },
                ],
                |result| {
                    result.unwrap();
                },
            )
            .provide_liquidity(
                creator.clone(),
                "uluna-uusd".to_string(),
                vec![
                    Coin {
                        denom: "uluna".to_string(),
                        amount: Uint128::from(1000000u128),
                    },
                    Coin {
                        denom: "uusd".to_string(),
                        amount: Uint128::from(1000000u128),
                    },
                ],
                |result| {
                    result.unwrap();
                },
            );

        // Lets try to add a swap route
        let swap_route_1 = SwapRoute {
            offer_asset_denom: "uwhale".to_string(),
            ask_asset_denom: "uusd".to_string(),
            swap_operations: vec![
                white_whale_std::pool_manager::SwapOperation::WhaleSwap {
                    token_in_denom: "uwhale".to_string(),
                    token_out_denom: "uluna".to_string(),
                    pool_identifier: "whale-uluna".to_string(),
                },
                white_whale_std::pool_manager::SwapOperation::WhaleSwap {
                    token_in_denom: "uluna".to_string(),
                    token_out_denom: "uusd".to_string(),
                    pool_identifier: "uluna-uusd".to_string(),
                },
            ],
        };

        suite.add_swap_routes(creator.clone(), vec![swap_route_1.clone()], |result| {
            assert!(result.unwrap().events.into_iter().any(|attr| {
                attr.attributes
                    .iter()
                    .any(|attr| attr.value == "add_swap_routes")
            }));
        });

        // Re-add the same swap route should fail
        suite.add_swap_routes(creator.clone(), vec![swap_route_1.clone()], |result| {
            assert_eq!(
                result.unwrap_err().downcast_ref::<ContractError>(),
                Some(&ContractError::SwapRouteAlreadyExists {
                    offer_asset: "uwhale".to_string(),
                    ask_asset: "uusd".to_string()
                })
            );
        });

        // Let's query all swap routes
        suite.query_swap_routes(|result| {
            assert_eq!(result.unwrap().swap_routes[0], swap_route_1);
        });

        // Let;s query for the swap route creator
        suite.query_swap_route_creator(
            "uwhale".to_string(),
            "uusd".to_string(),
            |result: Result<SwapRouteCreatorResponse, _>| {
                assert_eq!(result.unwrap().creator, creator);
            },
        );
    }

    #[test]
    fn remove_swap_routes() {
        let mut suite = TestingSuite::default_with_balances(vec![
            coin(1_000_000_001u128, "uwhale".to_string()),
            coin(1_000_000_000u128, "uluna".to_string()),
            coin(1_000_000_001u128, "uusd".to_string()),
        ]);
        let creator = suite.creator();
        let other = suite.senders[1].clone();
        let unauthorized = suite.senders[2].clone();

        let first_pair = vec!["uwhale".to_string(), "uluna".to_string()];
        let second_pair = vec!["uluna".to_string(), "uusd".to_string()];

        #[cfg(not(feature = "osmosis"))]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::bps(50), // 0.5%
            },
            swap_fee: Fee {
                share: Decimal::bps(50), // 0.5%
            },
            burn_fee: Fee {
                share: Decimal::bps(50), // 0.5%
            },
            extra_fees: vec![],
        };
        #[cfg(feature = "osmosis")]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::bps(50),
            },
            swap_fee: Fee {
                share: Decimal::bps(50),
            },
            burn_fee: Fee {
                share: Decimal::bps(50),
            },
            osmosis_fee: Fee {
                share: Decimal::bps(50),
            },
            extra_fees: vec![],
        };

        // Create a pair
        suite
            .instantiate_default()
            .create_pair(
                creator.clone(),
                first_pair,
                vec![6u8, 6u8],
                pool_fees.clone(),
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                Some("whale-uluna".to_string()),
                vec![coin(1000, "uusd")],
                |result| {
                    result.unwrap();
                },
            )
            .create_pair(
                creator.clone(),
                second_pair,
                vec![6u8, 6u8],
                pool_fees,
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                Some("uluna-uusd".to_string()),
                vec![coin(1000, "uusd")],
                |result| {
                    result.unwrap();
                },
            )
            .provide_liquidity(
                creator.clone(),
                "whale-uluna".to_string(),
                vec![
                    Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::from(1000000u128),
                    },
                    Coin {
                        denom: "uluna".to_string(),
                        amount: Uint128::from(1000000u128),
                    },
                ],
                |result| {
                    result.unwrap();
                },
            )
            .provide_liquidity(
                creator.clone(),
                "uluna-uusd".to_string(),
                vec![
                    Coin {
                        denom: "uluna".to_string(),
                        amount: Uint128::from(1000000u128),
                    },
                    Coin {
                        denom: "uusd".to_string(),
                        amount: Uint128::from(1000000u128),
                    },
                ],
                |result| {
                    result.unwrap();
                },
            );

        // Lets try to add a swap route
        let swap_route_1 = SwapRoute {
            offer_asset_denom: "uwhale".to_string(),
            ask_asset_denom: "uusd".to_string(),
            swap_operations: vec![
                white_whale_std::pool_manager::SwapOperation::WhaleSwap {
                    token_in_denom: "uwhale".to_string(),
                    token_out_denom: "uluna".to_string(),
                    pool_identifier: "whale-uluna".to_string(),
                },
                white_whale_std::pool_manager::SwapOperation::WhaleSwap {
                    token_in_denom: "uluna".to_string(),
                    token_out_denom: "uusd".to_string(),
                    pool_identifier: "uluna-uusd".to_string(),
                },
            ],
        };

        suite.add_swap_routes(other.clone(), vec![swap_route_1.clone()], |result| {
            assert!(result.unwrap().events.into_iter().any(|attr| {
                attr.attributes
                    .iter()
                    .any(|attr| attr.value == "add_swap_routes")
            }));
        });

        // Let's query for all swap routes
        suite.query_swap_routes(|result| {
            assert_eq!(result.unwrap().swap_routes[0], swap_route_1.clone());
        });

        // Let's query for the swap route
        suite.query_swap_route("uwhale".to_string(), "uusd".to_string(), |result| {
            assert_eq!(
                result.unwrap().swap_route,
                SwapRoute {
                    offer_asset_denom: "uwhale".to_string(),
                    ask_asset_denom: "uusd".to_string(),
                    swap_operations: swap_route_1.swap_operations.clone(),
                }
            );
        });

        // Let;s query for the swap route creator
        suite.query_swap_route_creator(
            "uwhale".to_string(),
            "uusd".to_string(),
            |result: Result<SwapRouteCreatorResponse, _>| {
                assert_eq!(result.unwrap().creator, other);
            },
        );

        // Removing a swap route as a non-route-creator & non-admin should fail
        suite.remove_swap_routes(unauthorized.clone(), vec![swap_route_1.clone()], |result| {
            assert!(result.is_err());
        });

        // Lets try to remove the swap route as the swap route creator
        suite.remove_swap_routes(other.clone(), vec![swap_route_1.clone()], |result| {
            assert!(result.unwrap().events.into_iter().any(|attr| {
                attr.attributes
                    .iter()
                    .any(|attr| attr.value == "remove_swap_routes")
            }));
        });

        // Let's query for all swap routes
        suite.query_swap_routes(|result| {
            assert_eq!(result.unwrap().swap_routes.len(), 0);
        });

        // Re-remove the same swap route should fail
        suite.remove_swap_routes(creator.clone(), vec![swap_route_1.clone()], |result| {
            assert_eq!(
                result.unwrap_err().downcast_ref::<ContractError>(),
                Some(&ContractError::NoSwapRouteForAssets {
                    offer_asset: "uwhale".to_string(),
                    ask_asset: "uusd".to_string()
                })
            );
        });
    }
}

mod swapping {
    use std::cell::RefCell;

    use cosmwasm_std::assert_approx_eq;

    use super::*;

    #[test]
    fn basic_swapping_test() {
        let mut suite = TestingSuite::default_with_balances(vec![
            coin(1_000_000_001u128, "uwhale".to_string()),
            coin(1_000_000_000u128, "uluna".to_string()),
            coin(1_000_000_001u128, "uusd".to_string()),
        ]);
        let creator = suite.creator();
        let _other = suite.senders[1].clone();
        let _unauthorized = suite.senders[2].clone();
        // Asset infos with uwhale and uluna

        let asset_infos = vec!["uwhale".to_string(), "uluna".to_string()];

        // Default Pool fees white_whale_std::pool_network::pair::PoolFee
        // Protocol fee is 0.01% and swap fee is 0.02% and burn fee is 0%
        #[cfg(not(feature = "osmosis"))]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::from_ratio(1u128, 100_000u128),
            },
            swap_fee: Fee {
                share: Decimal::from_ratio(1u128, 100_000u128),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
            extra_fees: vec![],
        };

        #[cfg(feature = "osmosis")]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::from_ratio(1u128, 100_000u128),
            },
            swap_fee: Fee {
                share: Decimal::from_ratio(1u128, 100_000u128),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
            osmosis_fee: Fee {
                share: Decimal::from_ratio(1u128, 100_000u128),
            },
            extra_fees: vec![],
        };

        // Create a pair
        suite.instantiate_default().create_pair(
            creator.clone(),
            asset_infos,
            vec![6u8, 6u8],
            pool_fees,
            white_whale_std::pool_network::asset::PairType::ConstantProduct,
            Some("whale-uluna".to_string()),
            vec![coin(1000, "uusd")],
            |result| {
                result.unwrap();
            },
        );

        // Query pair info to ensure the query is working fine
        suite.query_pair_info("whale-uluna".to_string(), |result| {
            assert_eq!(result.unwrap().pair_info.asset_decimals, vec![6u8, 6u8]);
        });

        // Lets try to add liquidity
        suite.provide_liquidity(
            creator.clone(),
            "whale-uluna".to_string(),
            None,
            None,
            vec![
                Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::from(1000000u128),
                },
                Coin {
                    denom: "uluna".to_string(),
                    amount: Uint128::from(1000000u128),
                },
            ],
            |result| {
                // Ensure we got 999_000 in the response which is 1mil less the initial liquidity amount
                assert!(result.unwrap().events.iter().any(|event| {
                    event.attributes.iter().any(|attr| {
                        attr.key == "share"
                            && attr.value
                                == (Uint128::from(1_000_000u128) - MINIMUM_LIQUIDITY_AMOUNT)
                                    .to_string()
                    })
                }));
            },
        );

        let simulated_return_amount = RefCell::new(Uint128::zero());
        suite.query_simulation(
            "whale-uluna".to_string(),
            Coin {
                denom: "uwhale".to_string(),
                amount: Uint128::from(1000u128),
            },
            |result| {
                // Ensure that the return amount is 1_000 minus spread
                assert_eq!(
                    result.as_ref().unwrap().return_amount + result.as_ref().unwrap().spread_amount,
                    Uint128::from(1000u128)
                );
                *simulated_return_amount.borrow_mut() = result.unwrap().return_amount;
            },
        );

        // Now lets try a swap
        suite.swap(
            creator.clone(),
            Coin {
                denom: "uwhale".to_string(),
                amount: Uint128::from(1000u128),
            },
            "uluna".to_string(),
            None,
            None,
            None,
            "whale-uluna".to_string(),
            vec![coin(1000u128, "uwhale".to_string())],
            |result| {
                // Find the key with 'offer_amount' and the key with 'return_amount'
                // Ensure that the offer amount is 1000 and the return amount is greater than 0
                let mut return_amount = String::new();
                let mut offer_amount = String::new();

                for event in result.unwrap().events {
                    if event.ty == "wasm" {
                        for attribute in event.attributes {
                            match attribute.key.as_str() {
                                "return_amount" => return_amount = attribute.value,
                                "offer_amount" => offer_amount = attribute.value,
                                _ => {}
                            }
                        }
                    }
                }
                // Because the Pool was created and 1_000_000 of each token has been provided as liquidity
                // Assuming no fees we should expect a small swap of 1000 to result in not too much slippage
                // Expect 1000 give or take 0.002 difference
                // Once fees are added and being deducted properly only the "0.002" should be changed.
                assert_approx_eq!(
                    offer_amount.parse::<u128>().unwrap(),
                    return_amount.parse::<u128>().unwrap(),
                    "0.002"
                );
                assert_approx_eq!(
                    simulated_return_amount.borrow().u128(),
                    return_amount.parse::<u128>().unwrap(),
                    "0.002"
                );
            },
        );

        let simulated_offer_amount = RefCell::new(Uint128::zero());
        suite.query_reverse_simulation(
            "whale-uluna".to_string(),
            "uluna".to_string(),
            Coin {
                denom: "uwhale".to_string(),
                amount: Uint128::from(1000u128),
            },
            |result| {
                println!("{:?}", result);
                *simulated_offer_amount.borrow_mut() = result.unwrap().offer_amount;
            },
        );
        // Another swap but this time the other way around
        // Now lets try a swap
        suite.swap(
            creator.clone(),
            Coin {
                denom: "uluna".to_string(),
                amount: Uint128::from(simulated_offer_amount.borrow().u128()),
            },
            "uwhale".to_string(),
            None,
            None,
            None,
            "whale-uluna".to_string(),
            vec![coin(
                simulated_offer_amount.borrow().u128(),
                "uluna".to_string(),
            )],
            |result| {
                // Find the key with 'offer_amount' and the key with 'return_amount'
                // Ensure that the offer amount is 1000 and the return amount is greater than 0
                let mut return_amount = String::new();
                let mut offer_amount = String::new();

                for event in result.unwrap().events {
                    println!("{:?}", event);
                    if event.ty == "wasm" {
                        for attribute in event.attributes {
                            match attribute.key.as_str() {
                                "return_amount" => return_amount = attribute.value,
                                "offer_amount" => offer_amount = attribute.value,
                                _ => {}
                            }
                        }
                    }
                }
                assert_approx_eq!(
                    simulated_offer_amount.borrow().u128(),
                    offer_amount.parse::<u128>().unwrap(),
                    "0.002"
                );

                assert_approx_eq!(1000u128, return_amount.parse::<u128>().unwrap(), "0.003");
            },
        );
    }

    #[test]
    fn basic_swapping_test_stable_swap() {
        let mut suite = TestingSuite::default_with_balances(vec![
            coin(1_000_000_001u128, "uwhale".to_string()),
            coin(1_000_000_000u128, "uluna".to_string()),
            coin(1_000_000_001u128, "uusd".to_string()),
        ]);
        let creator = suite.creator();
        let _other = suite.senders[1].clone();
        let _unauthorized = suite.senders[2].clone();
        // Asset infos with uwhale and uluna

        let asset_infos = vec!["uwhale".to_string(), "uluna".to_string()];

        // Default Pool fees white_whale_std::pool_network::pair::PoolFee
        // Protocol fee is 0.01% and swap fee is 0.02% and burn fee is 0%
        #[cfg(not(feature = "osmosis"))]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::from_ratio(1u128, 1000u128),
            },
            swap_fee: Fee {
                share: Decimal::from_ratio(1u128, 10_000_u128),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
            extra_fees: vec![],
        };

        #[cfg(feature = "osmosis")]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::from_ratio(1u128, 1000u128),
            },
            swap_fee: Fee {
                share: Decimal::from_ratio(1u128, 100_00u128),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
            osmosis_fee: Fee {
                share: Decimal::from_ratio(1u128, 1000u128),
            },
            extra_fees: vec![],
        };

        // Create a pair
        suite.instantiate_default().create_pair(
            creator.clone(),
            asset_infos,
            vec![6u8, 6u8],
            pool_fees,
            white_whale_std::pool_network::asset::PairType::StableSwap { amp: 100 },
            Some("whale-uluna".to_string()),
            vec![coin(1000, "uusd")],
            |result| {
                result.unwrap();
            },
        );

        // Lets try to add liquidity
        suite.provide_liquidity(
            creator.clone(),
            "whale-uluna".to_string(),
            None,
            None,
            vec![
                Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::from(1000000u128),
                },
                Coin {
                    denom: "uluna".to_string(),
                    amount: Uint128::from(1000000u128),
                },
            ],
            |result| {
                // Ensure we got 999000 in the response which is 1mil less the initial liquidity amount
                for event in result.unwrap().events {
                    println!("{:?}", event);
                }
            },
        );
        let simulated_return_amount = RefCell::new(Uint128::zero());
        suite.query_simulation(
            "whale-uluna".to_string(),
            Coin {
                denom: "uwhale".to_string(),
                amount: Uint128::from(1000u128),
            },
            |result| {
                println!("{:?}", result);
                *simulated_return_amount.borrow_mut() = result.unwrap().return_amount;
            },
        );

        // Now lets try a swap
        suite.swap(
            creator.clone(),
            Coin {
                denom: "uwhale".to_string(),
                amount: Uint128::from(1000u128),
            },
            "uluna".to_string(),
            None,
            None,
            None,
            "whale-uluna".to_string(),
            vec![coin(1000u128, "uwhale".to_string())],
            |result| {
                // Find the key with 'offer_amount' and the key with 'return_amount'
                // Ensure that the offer amount is 1000 and the return amount is greater than 0
                let mut return_amount = String::new();
                let mut offer_amount = String::new();

                for event in result.unwrap().events {
                    if event.ty == "wasm" {
                        for attribute in event.attributes {
                            match attribute.key.as_str() {
                                "return_amount" => return_amount = attribute.value,
                                "offer_amount" => offer_amount = attribute.value,
                                _ => {}
                            }
                        }
                    }
                }
                // Because the Pool was created and 1_000_000 of each token has been provided as liquidity
                // Assuming no fees we should expect a small swap of 1000 to result in not too much slippage
                // Expect 1000 give or take 0.002 difference
                // Once fees are added and being deducted properly only the "0.002" should be changed.
                assert_approx_eq!(
                    offer_amount.parse::<u128>().unwrap(),
                    return_amount.parse::<u128>().unwrap(),
                    "0.002"
                );
                assert_approx_eq!(
                    simulated_return_amount.borrow().u128(),
                    return_amount.parse::<u128>().unwrap(),
                    "0.002"
                );
            },
        );

        let simulated_offer_amount = RefCell::new(Uint128::zero());
        suite.query_reverse_simulation(
            "whale-uluna".to_string(),
            "uluna".to_string(),
            Coin {
                denom: "uwhale".to_string(),
                amount: Uint128::from(1000u128),
            },
            |result| {
                println!("{:?}", result);
                *simulated_offer_amount.borrow_mut() = result.unwrap().offer_amount;
            },
        );
        // Another swap but this time the other way around
        // Now lets try a swap
        suite.swap(
            creator.clone(),
            Coin {
                denom: "uluna".to_string(),
                amount: Uint128::from(simulated_offer_amount.borrow().u128()),
            },
            "uwhale".to_string(),
            None,
            None,
            None,
            "whale-uluna".to_string(),
            vec![coin(
                simulated_offer_amount.borrow().u128(),
                "uluna".to_string(),
            )],
            |result| {
                // Find the key with 'offer_amount' and the key with 'return_amount'
                // Ensure that the offer amount is 1000 and the return amount is greater than 0
                let mut return_amount = String::new();
                let mut offer_amount = String::new();

                for event in result.unwrap().events {
                    println!("{:?}", event);
                    if event.ty == "wasm" {
                        for attribute in event.attributes {
                            match attribute.key.as_str() {
                                "return_amount" => return_amount = attribute.value,
                                "offer_amount" => offer_amount = attribute.value,
                                _ => {}
                            }
                        }
                    }
                }
                assert_approx_eq!(
                    simulated_offer_amount.borrow().u128(),
                    offer_amount.parse::<u128>().unwrap(),
                    "0.002"
                );

                assert_approx_eq!(1000u128, return_amount.parse::<u128>().unwrap(), "0.003");
            },
        );
    }

    #[test]
    fn swap_with_fees() {
        let mut suite = TestingSuite::default_with_balances(vec![
            coin(1_000_000_000_001u128, "uwhale".to_string()),
            coin(1_000_000_000_000u128, "uluna".to_string()),
            coin(1_000_000_000_001u128, "uusd".to_string()),
        ]);
        let creator = suite.creator();
        let _other = suite.senders[1].clone();
        let _unauthorized = suite.senders[2].clone();
        // Asset infos with uwhale and uluna

        let asset_infos = vec!["uwhale".to_string(), "uluna".to_string()];

        // Default Pool fees white_whale_std::pool_network::pair::PoolFee
        // Protocol fee is 0.001% and swap fee is 0.002% and burn fee is 0%
        #[cfg(not(feature = "osmosis"))]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::from_ratio(1u128, 100_000u128),
            },
            swap_fee: Fee {
                share: Decimal::from_ratio(2u128, 100_000u128),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
            extra_fees: vec![],
        };

        #[cfg(feature = "osmosis")]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::from_ratio(1u128, 100_000u128),
            },
            swap_fee: Fee {
                share: Decimal::from_ratio(2u128, 100_000u128),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
            osmosis_fee: Fee {
                share: Decimal::zero(),
            },
            extra_fees: vec![],
        };

        // Create a pair
        suite.instantiate_default().create_pair(
            creator.clone(),
            asset_infos,
            vec![6u8, 6u8],
            pool_fees,
            white_whale_std::pool_network::asset::PairType::ConstantProduct,
            Some("whale-uluna".to_string()),
            vec![coin(1000, "uusd")],
            |result| {
                result.unwrap();
            },
        );

        // Lets try to add liquidity, 1000 of each token.
        suite.provide_liquidity(
            creator.clone(),
            "whale-uluna".to_string(),
            None,
            None,
            vec![
                Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::from(1000_000000u128),
                },
                Coin {
                    denom: "uluna".to_string(),
                    amount: Uint128::from(1000_000000u128),
                },
            ],
            |result| {
                // Ensure we got 999000 in the response which is 1mil less the initial liquidity amount
                for event in result.unwrap().events {
                    println!("{:?}", event);
                }
            },
        );

        // Now lets try a swap, max spread is set to 1%
        // With 1000 of each token and a swap of 10 WHALE
        // We should expect a return of 9900792 of ULUNA
        // Applying Fees on the swap:
        //    - Protocol Fee: 0.001% on uLUNA -> 99.
        //    - Swap Fee: 0.002% on uLUNA -> 198.
        // Total Fees: 297 uLUNA

        // Spread Amount: 99,010 uLUNA.
        // Swap Fee Amount: 198 uLUNA.
        // Protocol Fee Amount: 99 uLUNA.
        // Burn Fee Amount: 0 uLUNA (as expected since burn fee is set to 0%).
        // Total -> 9,900,693 (Returned Amount) + 99,010 (Spread)(0.009x%) + 198 (Swap Fee) + 99 (Protocol Fee) = 10,000,000 uLUNA
        suite.swap(
            creator.clone(),
            Coin {
                denom: "uwhale".to_string(),
                amount: Uint128::from(10000000u128),
            },
            "uluna".to_string(),
            None,
            Some(Decimal::percent(1)),
            None,
            "whale-uluna".to_string(),
            vec![coin(10000000u128, "uwhale".to_string())],
            |result| {
                // Find the key with 'offer_amount' and the key with 'return_amount'
                // Ensure that the offer amount is 1000 and the return amount is greater than 0
                let mut return_amount = String::new();
                let mut offer_amount = String::new();

                for event in result.unwrap().events {
                    println!("{:?}", event);
                    if event.ty == "wasm" {
                        for attribute in event.attributes {
                            match attribute.key.as_str() {
                                "return_amount" => return_amount = attribute.value,
                                "offer_amount" => offer_amount = attribute.value,
                                _ => {}
                            }
                        }
                    }
                }
                // Because the Pool was created and 1_000_000 of each token has been provided as liquidity
                // Assuming no fees we should expect a small swap of 1000 to result in not too much slippage
                // Expect 1000 give or take 0.002 difference
                // Once fees are added and being deducted properly only the "0.002" should be changed.
                assert_approx_eq!(
                    offer_amount.parse::<u128>().unwrap(),
                    return_amount.parse::<u128>().unwrap(),
                    "0.01"
                );
            },
        );

        // Verify fee collection by querying the address of the whale lair and checking its balance
        // Should be 297 uLUNA
        suite.query_balance(
            suite.bonding_manager_addr.to_string(),
            "uluna".to_string(),
            |result| {
                assert_eq!(result.unwrap().amount, Uint128::from(297u128));
            },
        );
    }
}

mod ownership {
    use white_whale_std::pool_manager::FeatureToggle;

    use super::*;

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
                        _ => {
                            panic!("Wrong error type, should return ContractError::OwnershipError")
                        }
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
    fn checks_ownership_when_updating_config() {
        let mut suite = TestingSuite::default_with_balances(vec![]);
        let unauthorized = suite.senders[2].clone();

        suite.instantiate_default().update_config(
            unauthorized.clone(),
            None,
            None,
            None,
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::OwnershipError { .. } => {}
                    _ => {
                        panic!("Wrong error type, should return ContractError::OwnershipError")
                    }
                }
            },
        );
    }

    #[test]
    fn updates_config_fields() {
        let mut suite = TestingSuite::default_with_balances(vec![]);
        let creator = suite.creator();
        let other = suite.senders[1].clone();

        suite.instantiate_default();
        let current_pool_creation_fee = suite.query_config().pool_creation_fee;
        let initial_config = suite.query_config();

        suite.update_config(
            creator,
            Some(other),
            Some(coin(
                current_pool_creation_fee
                    .amount
                    .checked_add(Uint128::from(1u32))
                    .unwrap()
                    .u128(),
                current_pool_creation_fee.denom,
            )),
            Some(FeatureToggle {
                deposits_enabled: false,
                swaps_enabled: false,
                withdrawals_enabled: false,
            }),
            |res| {
                res.unwrap();
            },
        );

        let config = suite.query_config();
        assert_ne!(
            config.bonding_manager_addr,
            initial_config.bonding_manager_addr
        );
        assert_ne!(config.pool_creation_fee, initial_config.pool_creation_fee);
        assert_ne!(config.feature_toggle, initial_config.feature_toggle);
    }
}

mod locking_lp {
    use cosmwasm_std::{coin, Coin, Decimal, Uint128};

    use white_whale_std::fee::{Fee, PoolFee};
    use white_whale_std::incentive_manager::Position;
    use white_whale_std::pool_network::asset::MINIMUM_LIQUIDITY_AMOUNT;

    use crate::tests::suite::TestingSuite;

    #[test]
    fn provide_liquidity_locking_lp_no_lock_position_identifier() {
        let mut suite = TestingSuite::default_with_balances(vec![
            coin(10_000_000u128, "uwhale".to_string()),
            coin(10_000_000u128, "uluna".to_string()),
            coin(10_000u128, "uusd".to_string()),
        ]);
        let creator = suite.creator();
        let _other = suite.senders[1].clone();
        let _unauthorized = suite.senders[2].clone();

        // Asset denoms with uwhale and uluna
        let asset_denoms = vec!["uwhale".to_string(), "uluna".to_string()];

        // Default Pool fees white_whale_std::pool_network::pair::PoolFee
        #[cfg(not(feature = "osmosis"))]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::zero(),
            },
            swap_fee: Fee {
                share: Decimal::zero(),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
            extra_fees: vec![],
        };

        #[cfg(feature = "osmosis")]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::zero(),
            },
            swap_fee: Fee {
                share: Decimal::zero(),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
            osmosis_fee: Fee {
                share: Decimal::zero(),
            },
            extra_fees: vec![],
        };

        // Create a pair
        suite.instantiate_default().create_pair(
            creator.clone(),
            asset_denoms,
            vec![6u8, 6u8],
            pool_fees,
            white_whale_std::pool_network::asset::PairType::ConstantProduct,
            Some("whale-uluna".to_string()),
            vec![coin(1000, "uusd")],
            |result| {
                result.unwrap();
            },
        );

        let contract_addr = suite.pool_manager_addr.clone();
        let incentive_manager_addr = suite.incentive_manager_addr.clone();
        let lp_denom = suite.get_lp_denom("whale-uluna".to_string());

        // Lets try to add liquidity
        suite
            .provide_liquidity(
                creator.clone(),
                "whale-uluna".to_string(),
                Some(86_400u64),
                None,
                vec![
                    Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::from(1_000_000u128),
                    },
                    Coin {
                        denom: "uluna".to_string(),
                        amount: Uint128::from(1_000_000u128),
                    },
                ],
                |result| {
                    // Ensure we got 999_000 in the response which is 1_000_000 less the initial liquidity amount
                    assert!(result.unwrap().events.iter().any(|event| {
                        event.attributes.iter().any(|attr| {
                            attr.key == "share"
                                && attr.value
                                    == (Uint128::from(1_000_000u128) - MINIMUM_LIQUIDITY_AMOUNT)
                                        .to_string()
                        })
                    }));
                },
            )
            .query_all_balances(creator.to_string(), |result| {
                let balances = result.unwrap();
                // the lp tokens should have gone to the incentive manager
                assert!(!balances
                    .iter()
                    .any(|coin| { coin.denom == lp_denom.clone() }));
            })
            // contract should have 1_000 LP shares (MINIMUM_LIQUIDITY_AMOUNT)
            .query_all_balances(contract_addr.to_string(), |result| {
                let balances = result.unwrap();
                assert!(balances.iter().any(|coin| {
                    coin.denom == lp_denom.clone() && coin.amount == MINIMUM_LIQUIDITY_AMOUNT
                }));
            })
            // check the LP went to the incentive manager
            .query_all_balances(incentive_manager_addr.to_string(), |result| {
                let balances = result.unwrap();
                assert!(balances.iter().any(|coin| {
                    coin.denom == lp_denom && coin.amount == Uint128::from(999_000u128)
                }));
            });

        suite.query_incentive_positions(creator.clone(), None, |result| {
            let positions = result.unwrap().positions;
            assert_eq!(positions.len(), 1);
            assert_eq!(positions[0], Position {
                identifier: "1".to_string(),
                lp_asset: Coin{ denom: "factory/migaloo1zwv6feuzhy6a9wekh96cd57lsarmqlwxdypdsplw6zhfncqw6ftqqhavvl/uwhale-uluna.pool.whale-uluna.uLP".to_string(), amount: Uint128::from(999_000u128) },
                unlocking_duration: 86_400,
                open: true,
                expiring_at: None,
                receiver: creator.clone(),
            });
        });

        // let's do it again, it should create another position on the incentive manager

        suite
            .provide_liquidity(
                creator.clone(),
                "whale-uluna".to_string(),
                Some(200_000u64),
                None,
                vec![
                    Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::from(1_000_000u128),
                    },
                    Coin {
                        denom: "uluna".to_string(),
                        amount: Uint128::from(1_000_000u128),
                    },
                ],
                |result| {
                    result.unwrap();
                },
            )
            .query_all_balances(creator.to_string(), |result| {
                let balances = result.unwrap();
                // the lp tokens should have gone to the incentive manager
                assert!(!balances
                    .iter()
                    .any(|coin| { coin.denom == lp_denom.clone() }));
            })
            // check the LP went to the incentive manager
            .query_all_balances(incentive_manager_addr.to_string(), |result| {
                let balances = result.unwrap();
                assert!(balances.iter().any(|coin| {
                    coin.denom == lp_denom && coin.amount == Uint128::from(1_999_000u128)
                }));
            });

        suite.query_incentive_positions(creator.clone(), None, |result| {
            let positions = result.unwrap().positions;
            assert_eq!(positions.len(), 2);
            assert_eq!(positions[0], Position {
                identifier: "1".to_string(),
                lp_asset: Coin{ denom: "factory/migaloo1zwv6feuzhy6a9wekh96cd57lsarmqlwxdypdsplw6zhfncqw6ftqqhavvl/uwhale-uluna.pool.whale-uluna.uLP".to_string(), amount: Uint128::from(999_000u128) },
                unlocking_duration: 86_400,
                open: true,
                expiring_at: None,
                receiver: creator.clone(),
            });
            assert_eq!(positions[1], Position {
                identifier: "2".to_string(),
                lp_asset: Coin{ denom: "factory/migaloo1zwv6feuzhy6a9wekh96cd57lsarmqlwxdypdsplw6zhfncqw6ftqqhavvl/uwhale-uluna.pool.whale-uluna.uLP".to_string(), amount: Uint128::from(1_000_000u128) },
                unlocking_duration: 200_000,
                open: true,
                expiring_at: None,
                receiver: creator.clone(),
            });
        });
    }

    #[test]
    fn provide_liquidity_locking_lp_reusing_position_identifier() {
        let mut suite = TestingSuite::default_with_balances(vec![
            coin(10_000_000u128, "uwhale".to_string()),
            coin(10_000_000u128, "uluna".to_string()),
            coin(10_000u128, "uusd".to_string()),
        ]);
        let creator = suite.creator();
        let _other = suite.senders[1].clone();
        let _unauthorized = suite.senders[2].clone();

        // Asset denoms with uwhale and uluna
        let asset_denoms = vec!["uwhale".to_string(), "uluna".to_string()];

        // Default Pool fees white_whale_std::pool_network::pair::PoolFee
        #[cfg(not(feature = "osmosis"))]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::zero(),
            },
            swap_fee: Fee {
                share: Decimal::zero(),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
            extra_fees: vec![],
        };

        #[cfg(feature = "osmosis")]
        let pool_fees = PoolFee {
            protocol_fee: Fee {
                share: Decimal::zero(),
            },
            swap_fee: Fee {
                share: Decimal::zero(),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
            osmosis_fee: Fee {
                share: Decimal::zero(),
            },
            extra_fees: vec![],
        };

        // Create a pair
        suite.instantiate_default().create_pair(
            creator.clone(),
            asset_denoms,
            vec![6u8, 6u8],
            pool_fees,
            white_whale_std::pool_network::asset::PairType::ConstantProduct,
            Some("whale-uluna".to_string()),
            vec![coin(1000, "uusd")],
            |result| {
                result.unwrap();
            },
        );

        let contract_addr = suite.pool_manager_addr.clone();
        let incentive_manager_addr = suite.incentive_manager_addr.clone();
        let lp_denom = suite.get_lp_denom("whale-uluna".to_string());

        // Lets try to add liquidity
        suite
            .provide_liquidity(
                creator.clone(),
                "whale-uluna".to_string(),
                Some(86_400u64),
                Some("incentive_identifier".to_string()),
                vec![
                    Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::from(1_000_000u128),
                    },
                    Coin {
                        denom: "uluna".to_string(),
                        amount: Uint128::from(1_000_000u128),
                    },
                ],
                |result| {
                    // Ensure we got 999_000 in the response which is 1_000_000 less the initial liquidity amount
                    assert!(result.unwrap().events.iter().any(|event| {
                        event.attributes.iter().any(|attr| {
                            attr.key == "share"
                                && attr.value
                                    == (Uint128::from(1_000_000u128) - MINIMUM_LIQUIDITY_AMOUNT)
                                        .to_string()
                        })
                    }));
                },
            )
            .query_all_balances(creator.to_string(), |result| {
                let balances = result.unwrap();
                // the lp tokens should have gone to the incentive manager
                assert!(!balances
                    .iter()
                    .any(|coin| { coin.denom == lp_denom.clone() }));
            })
            // contract should have 1_000 LP shares (MINIMUM_LIQUIDITY_AMOUNT)
            .query_all_balances(contract_addr.to_string(), |result| {
                let balances = result.unwrap();
                assert!(balances.iter().any(|coin| {
                    coin.denom == lp_denom.clone() && coin.amount == MINIMUM_LIQUIDITY_AMOUNT
                }));
            })
            // check the LP went to the incentive manager
            .query_all_balances(incentive_manager_addr.to_string(), |result| {
                let balances = result.unwrap();
                assert!(balances.iter().any(|coin| {
                    coin.denom == lp_denom && coin.amount == Uint128::from(999_000u128)
                }));
            });

        suite.query_incentive_positions(creator.clone(), None, |result| {
            let positions = result.unwrap().positions;
            assert_eq!(positions.len(), 1);
            assert_eq!(positions[0], Position {
                identifier: "incentive_identifier".to_string(),
                lp_asset: Coin{ denom: "factory/migaloo1zwv6feuzhy6a9wekh96cd57lsarmqlwxdypdsplw6zhfncqw6ftqqhavvl/uwhale-uluna.pool.whale-uluna.uLP".to_string(), amount: Uint128::from(999_000u128) },
                unlocking_duration: 86_400,
                open: true,
                expiring_at: None,
                receiver: creator.clone(),
            });
        });

        // let's do it again, reusing the same incentive identifier

        suite
            .provide_liquidity(
                creator.clone(),
                "whale-uluna".to_string(),
                Some(200_000u64),
                Some("incentive_identifier".to_string()),
                vec![
                    Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::from(1_000_000u128),
                    },
                    Coin {
                        denom: "uluna".to_string(),
                        amount: Uint128::from(1_000_000u128),
                    },
                ],
                |result| {
                    result.unwrap();
                },
            )
            .query_all_balances(creator.to_string(), |result| {
                let balances = result.unwrap();
                // the lp tokens should have gone to the incentive manager
                assert!(!balances
                    .iter()
                    .any(|coin| { coin.denom == lp_denom.clone() }));
            })
            // check the LP went to the incentive manager
            .query_all_balances(incentive_manager_addr.to_string(), |result| {
                let balances = result.unwrap();
                assert!(balances.iter().any(|coin| {
                    coin.denom == lp_denom && coin.amount == Uint128::from(1_999_000u128)
                }));
            });

        suite.query_incentive_positions(creator.clone(), None, |result| {
            let positions = result.unwrap().positions;
            // the position should be updated
            assert_eq!(positions.len(), 1);
            assert_eq!(positions[0], Position {
                identifier: "incentive_identifier".to_string(),
                lp_asset: Coin{ denom: "factory/migaloo1zwv6feuzhy6a9wekh96cd57lsarmqlwxdypdsplw6zhfncqw6ftqqhavvl/uwhale-uluna.pool.whale-uluna.uLP".to_string(), amount: Uint128::from(1_999_000u128) },
                unlocking_duration: 86_400,
                open: true,
                expiring_at: None,
                receiver: creator.clone(),
            });
        });
    }
}
