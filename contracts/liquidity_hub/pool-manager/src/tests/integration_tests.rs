use crate::ContractError;
use cosmwasm_std::{coin, Addr, Coin, Decimal, Uint128};
use white_whale_std::fee::Fee;
use white_whale_std::pool_network::asset::{Asset, AssetInfo, MINIMUM_LIQUIDITY_AMOUNT};
use white_whale_std::pool_network::pair::PoolFee;
use white_whale_std::vault_manager::LpTokenType;

use super::suite::TestingSuite;

// Using our suite lets test create pair
// and add liquidity to it

#[test]
fn instantiate_normal() {
    let mut suite = TestingSuite::default_with_balances(vec![]);

    suite.instantiate(
        suite.senders[0].to_string(),
        LpTokenType::TokenFactory,
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            amount: Uint128::new(1_000u128),
        },
    );

    let cw20_code_id = suite.create_cw20_token();
    suite.instantiate(
        suite.senders[0].to_string(),
        LpTokenType::Cw20(cw20_code_id),
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            amount: Uint128::new(1_000u128),
        },
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
fn deposit_and_withdraw_sanity_check() {
    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_001u128, "uwhale".to_string()),
        coin(1_000_000_000u128, "uluna".to_string()),
        coin(1_000_000_001u128, "uusd".to_string()),
    ]);
    let creator = suite.creator();
    let other = suite.senders[1].clone();
    let unauthorized = suite.senders[2].clone();
    // Asset infos with uwhale and uluna

    let asset_infos = vec![
        AssetInfo::NativeToken {
            denom: "uwhale".to_string(),
        },
        AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        },
    ];

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
    };

    // Create a pair
    suite
        .instantiate_with_cw20_lp_token()
        .add_native_token_decimals(creator.clone(), "uwhale".to_string(), 6)
        .add_native_token_decimals(creator.clone(), "uluna".to_string(), 6)
        .create_pair(
            creator.clone(),
            asset_infos,
            pool_fees,
            white_whale_std::pool_network::asset::PairType::ConstantProduct,
            false,
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
        vec![
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::from(1000000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                amount: Uint128::from(1000000u128),
            },
        ],
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

    suite.query_amount_of_lp_token("whale-uluna".to_string(), creator.to_string(), |result| {
        assert_eq!(
            result.unwrap(),
            Uint128::from(1000000u128) - MINIMUM_LIQUIDITY_AMOUNT
        );
    });
}

#[test]
fn deposit_and_withdraw_cw20() {
    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_001u128, "uwhale".to_string()),
        coin(1_000_000_000u128, "uluna".to_string()),
        coin(1_000_000_001u128, "uusd".to_string()),
    ]);
    let creator = suite.creator();
    let other = suite.senders[1].clone();
    let unauthorized = suite.senders[2].clone();
    // Asset infos with uwhale and cw20

    let cw20_code_id = suite.create_cw20_token();

    let asset_infos = vec![
        AssetInfo::NativeToken {
            denom: "uwhale".to_string(),
        },
        AssetInfo::Token {
            contract_addr: suite.cw20_tokens[0].to_string(),
        },
    ];

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
    };

    // Create a pair
    suite
        .instantiate_with_cw20_lp_token()
        .add_native_token_decimals(creator.clone(), "uwhale".to_string(), 6)
        .create_pair(
            creator.clone(),
            asset_infos,
            pool_fees,
            white_whale_std::pool_network::asset::PairType::ConstantProduct,
            false,
            None,
            vec![coin(1000, "uusd")],
            |result| {
                result.unwrap();
            },
        );
    suite.increase_allowance(
        creator.clone(),
        suite.cw20_tokens[0].clone(),
        Uint128::from(1000000u128),
        suite.pool_manager_addr.clone(),
    );
    // Lets try to add liquidity
    suite.provide_liquidity(
        creator.clone(),
        "0".to_string(),
        vec![
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::from(1000000u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: suite.cw20_tokens[0].to_string(),
                },
                amount: Uint128::from(1000000u128),
            },
        ],
        vec![Coin {
            denom: "uwhale".to_string(),
            amount: Uint128::from(1000000u128),
        }],
        |result| {
            // Ensure we got 999000 in the response which is 1mil less the initial liquidity amount
            for event in result.unwrap().events {
                println!("{:?}", event);
            }
        },
    );

    suite.query_amount_of_lp_token("0".to_string(), creator.to_string(), |result| {
        assert_eq!(
            result.unwrap(),
            Uint128::from(1000000u128) - MINIMUM_LIQUIDITY_AMOUNT
        );
    });
    let assets = vec![
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            amount: Uint128::from(1000000u128),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: suite.cw20_tokens[0].to_string(),
            },
            amount: Uint128::from(1000000u128),
        },
    ];

    let lp_token = suite.query_lp_token("0".to_string(), creator.to_string());
    let lp_token_addr = match lp_token {
        AssetInfo::Token { contract_addr } => contract_addr,
        _ => {
            panic!("Liquidity token is not a cw20 token")
        }
    };
    suite.withdraw_liquidity_cw20(
        creator.clone(),
        "0".to_string(),
        assets,
        Uint128::from(1000000u128) - MINIMUM_LIQUIDITY_AMOUNT,
        Addr::unchecked(lp_token_addr),
        |result| {
            println!("{:?}", result);
            for event in result.unwrap().events {
                println!("{:?}", event);
            }
        },
    );
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
        let other = suite.senders[1].clone();
        let unauthorized = suite.senders[2].clone();
        // Asset infos with uwhale and cw20

        let cw20_code_id = suite.create_cw20_token();

        let asset_infos = vec![
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            AssetInfo::Token {
                contract_addr: suite.cw20_tokens[0].to_string(),
            },
        ];

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
        };
        // Create a pair
        suite
            .instantiate_with_cw20_lp_token()
            .add_native_token_decimals(creator.clone(), "uwhale".to_string(), 6)
            .create_pair(
                creator.clone(),
                asset_infos,
                pool_fees,
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                false,
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

        let _cw20_code_id = suite.create_cw20_token();

        let asset_infos = vec![
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            AssetInfo::Token {
                contract_addr: suite.cw20_tokens[0].to_string(),
            },
        ];

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
        };

        // Create a pair
        suite
            .instantiate_with_cw20_lp_token()
            .add_native_token_decimals(creator.clone(), "uwhale".to_string(), 6)
            .create_pair(
                creator.clone(),
                asset_infos.clone(),
                pool_fees.clone(),
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                false,
                Some("mycoolpair".to_string()),
                vec![coin(1000, "uusd")],
                |result| {
                    result.unwrap();
                },
            )
            .create_pair(
                creator.clone(),
                asset_infos,
                pool_fees,
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                false,
                Some("mycoolpair".to_string()),
                vec![coin(1000, "uusd")],
                |result| {
                    let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                    println!("{:?}", err);
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

    use super::*;
    #[test]
    fn basic_swap_operations_test() {
        let mut suite = TestingSuite::default_with_balances(vec![
            coin(1_000_000_001u128, "uwhale".to_string()),
            coin(1_000_000_000u128, "uluna".to_string()),
            coin(1_000_000_001u128, "uusd".to_string()),
        ]);
        let creator = suite.creator();
        let _other = suite.senders[1].clone();
        let _unauthorized = suite.senders[2].clone();
        // Asset infos with uwhale and uluna

        let first_pair = vec![
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
        ];

        let second_pair = vec![
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
        ];

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
        };

        // Create a pair
        suite
            .instantiate_with_cw20_lp_token()
            .add_native_token_decimals(creator.clone(), "uwhale".to_string(), 6)
            .add_native_token_decimals(creator.clone(), "uluna".to_string(), 6)
            .add_native_token_decimals(creator.clone(), "uusd".to_string(), 6)
            .create_pair(
                creator.clone(),
                first_pair,
                pool_fees.clone(),
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                false,
                Some("whale-uluna".to_string()),
                vec![coin(1000, "uusd")],
                |result| {
                    result.unwrap();
                },
            )
            .create_pair(
                creator.clone(),
                second_pair,
                pool_fees,
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                false,
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
            vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string(),
                    },
                    amount: Uint128::from(1000000u128),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    amount: Uint128::from(1000000u128),
                },
            ],
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
            vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    amount: Uint128::from(1000000u128),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    amount: Uint128::from(1000000u128),
                },
            ],
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
                token_in_info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                token_out_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                pool_identifier: "whale-uluna".to_string(),
            },
            white_whale_std::pool_manager::SwapOperation::WhaleSwap {
                token_in_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                token_out_info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                pool_identifier: "uluna-uusd".to_string(),
            },
        ];

        // before swap uusd balance = 1_000_000_001
        // - 2*1_000 pair creation fee
        // - 1_000_000 liquidity provision
        // - 1 for native token creation (for decimal precisions)
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
            suite.whale_lair_addr.to_string(),
            "uusd".to_string(),
            |amt| {
                assert_eq!(amt.unwrap().amount.u128(), 2000 + 4 * 2);
            },
        );
        suite.query_balance(
            suite.whale_lair_addr.to_string(),
            "uwhale".to_string(),
            |amt| {
                assert_eq!(amt.unwrap().amount.u128(), 0);
            },
        );
        suite.query_balance(
            suite.whale_lair_addr.to_string(),
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

        let first_pair = vec![
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
        ];

        let second_pair = vec![
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
        ];

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
        };

        // Create a pair
        suite
            .instantiate_with_cw20_lp_token()
            .add_native_token_decimals(creator.clone(), "uwhale".to_string(), 6)
            .add_native_token_decimals(creator.clone(), "uluna".to_string(), 6)
            .add_native_token_decimals(creator.clone(), "uusd".to_string(), 6)
            .create_pair(
                creator.clone(),
                first_pair,
                pool_fees.clone(),
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                false,
                Some("whale-uluna".to_string()),
                vec![coin(1000, "uusd")],
                |result| {
                    result.unwrap();
                },
            )
            .create_pair(
                creator.clone(),
                second_pair,
                pool_fees,
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                false,
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
            vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string(),
                    },
                    amount: Uint128::from(1000000u128),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    amount: Uint128::from(1000000u128),
                },
            ],
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
            vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    amount: Uint128::from(1000000u128),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    amount: Uint128::from(1000000u128),
                },
            ],
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

        let first_pair = vec![
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
        ];

        let second_pair = vec![
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
        ];

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
        };

        // Create a pair
        suite
            .instantiate_with_cw20_lp_token()
            .add_native_token_decimals(creator.clone(), "uwhale".to_string(), 6)
            .add_native_token_decimals(creator.clone(), "uluna".to_string(), 6)
            .add_native_token_decimals(creator.clone(), "uusd".to_string(), 6)
            .create_pair(
                creator.clone(),
                first_pair,
                pool_fees.clone(),
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                false,
                Some("whale-uluna".to_string()),
                vec![coin(1000, "uusd")],
                |result| {
                    result.unwrap();
                },
            )
            .create_pair(
                creator.clone(),
                second_pair,
                pool_fees,
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                false,
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
            vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string(),
                    },
                    amount: Uint128::from(1000000u128),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    amount: Uint128::from(1000000u128),
                },
            ],
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
            vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    amount: Uint128::from(1000000u128),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    amount: Uint128::from(1000000u128),
                },
            ],
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
                token_in_info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                token_out_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                pool_identifier: "whale-uluna".to_string(),
            },
            white_whale_std::pool_manager::SwapOperation::WhaleSwap {
                token_in_info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                token_out_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
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
                        previous_output: AssetInfo::NativeToken {
                            denom: "uluna".to_string()
                        },
                        next_input: AssetInfo::NativeToken {
                            denom: "uwhale".to_string()
                        }
                    })
                );
            },
        );
    }

    #[test]
    fn sends_to_correct_receiver() {
        let mut suite = TestingSuite::default_with_balances(vec![
            coin(1_000_000_001u128, "uwhale".to_string()),
            coin(1_000_000_000u128, "uluna".to_string()),
            coin(1_000_000_001u128, "uusd".to_string()),
        ]);
        let creator = suite.creator();
        let other = suite.senders[1].clone();
        let unauthorized = suite.senders[2].clone();
        // Asset infos with uwhale and uluna

        let first_pair = vec![
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
        ];

        let second_pair = vec![
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
        ];

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
        };

        // Create a pair
        suite
            .instantiate_with_cw20_lp_token()
            .add_native_token_decimals(creator.clone(), "uwhale".to_string(), 6)
            .add_native_token_decimals(creator.clone(), "uluna".to_string(), 6)
            .add_native_token_decimals(creator.clone(), "uusd".to_string(), 6)
            .create_pair(
                creator.clone(),
                first_pair,
                pool_fees.clone(),
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                false,
                Some("whale-uluna".to_string()),
                vec![coin(1000, "uusd")],
                |result| {
                    result.unwrap();
                },
            )
            .create_pair(
                creator.clone(),
                second_pair,
                pool_fees,
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                false,
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
            vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string(),
                    },
                    amount: Uint128::from(liquidity_amount),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    amount: Uint128::from(liquidity_amount),
                },
            ],
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
            vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    amount: Uint128::from(liquidity_amount),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    amount: Uint128::from(liquidity_amount),
                },
            ],
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
                token_in_info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                token_out_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                pool_identifier: "whale-uluna".to_string(),
            },
            white_whale_std::pool_manager::SwapOperation::WhaleSwap {
                token_in_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                token_out_info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                pool_identifier: "uluna-uusd".to_string(),
            },
        ];

        // before swap uusd balance = 1_000_000_001
        // before swap uwhale balance = 1_000_000_001
        // before swap uluna balance = 1_000_000_001
        let pre_swap_amount = 1_000_000_001;
        suite.query_balance(other.to_string(), "uusd".to_string(), |amt| {
            assert_eq!(amt.unwrap().amount.u128(), pre_swap_amount);
        });
        suite.query_balance(other.to_string(), "uwhale".to_string(), |amt| {
            assert_eq!(amt.unwrap().amount.u128(), pre_swap_amount);
        });
        suite.query_balance(other.to_string(), "uluna".to_string(), |amt| {
            assert_eq!(amt.unwrap().amount.u128(), pre_swap_amount - 1);
        });
        // also check the same for unauthorized receiver
        suite.query_balance(other.to_string(), "uusd".to_string(), |amt| {
            assert_eq!(amt.unwrap().amount.u128(), pre_swap_amount);
        });
        suite.query_balance(other.to_string(), "uwhale".to_string(), |amt| {
            assert_eq!(amt.unwrap().amount.u128(), pre_swap_amount);
        });
        suite.query_balance(other.to_string(), "uluna".to_string(), |amt| {
            assert_eq!(amt.unwrap().amount.u128(), pre_swap_amount - 1);
        });
        // also check for contract
        // when we add tokens to the contract, we must send a fee of 1_u128 so the contract
        // can register the native token
        suite.query_balance(
            suite.pool_manager_addr.to_string(),
            "uusd".to_string(),
            |amt| {
                assert_eq!(amt.unwrap().amount.u128(), liquidity_amount + 1);
            },
        );
        suite.query_balance(
            suite.pool_manager_addr.to_string(),
            "uwhale".to_string(),
            |amt| {
                assert_eq!(amt.unwrap().amount.u128(), liquidity_amount + 1);
            },
        );
        suite.query_balance(
            suite.pool_manager_addr.to_string(),
            "uluna".to_string(),
            |amt| {
                assert_eq!(amt.unwrap().amount.u128(), 2 * liquidity_amount + 1);
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
                assert_eq!(amt.unwrap().amount.u128(), liquidity_amount - 998 + 1);
            },
        );
        suite.query_balance(
            suite.pool_manager_addr.to_string(),
            "uwhale".to_string(),
            |amt| {
                assert_eq!(amt.unwrap().amount.u128(), liquidity_amount + 1000 + 1);
            },
        );
        suite.query_balance(
            suite.pool_manager_addr.to_string(),
            "uluna".to_string(),
            |amt| {
                assert_eq!(amt.unwrap().amount.u128(), 2 * liquidity_amount + 1);
            },
        );
    }

    #[test]
    fn checks_minimum_receive() {
        let mut suite = TestingSuite::default_with_balances(vec![
            coin(1_000_000_001u128, "uwhale".to_string()),
            coin(1_000_000_000u128, "uluna".to_string()),
            coin(1_000_000_001u128, "uusd".to_string()),
        ]);
        let creator = suite.creator();
        let _other = suite.senders[1].clone();
        let _unauthorized = suite.senders[2].clone();
        // Asset infos with uwhale and uluna

        let first_pair = vec![
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
        ];

        let second_pair = vec![
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
        ];

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
        };

        // Create a pair
        suite
            .instantiate_with_cw20_lp_token()
            .add_native_token_decimals(creator.clone(), "uwhale".to_string(), 6)
            .add_native_token_decimals(creator.clone(), "uluna".to_string(), 6)
            .add_native_token_decimals(creator.clone(), "uusd".to_string(), 6)
            .create_pair(
                creator.clone(),
                first_pair,
                pool_fees.clone(),
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                false,
                Some("whale-uluna".to_string()),
                vec![coin(1000, "uusd")],
                |result| {
                    result.unwrap();
                },
            )
            .create_pair(
                creator.clone(),
                second_pair,
                pool_fees,
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                false,
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
            vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string(),
                    },
                    amount: Uint128::from(1000000u128),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    amount: Uint128::from(1000000u128),
                },
            ],
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
            vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    amount: Uint128::from(1000000u128),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    amount: Uint128::from(1000000u128),
                },
            ],
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
                token_in_info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                token_out_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                pool_identifier: "whale-uluna".to_string(),
            },
            white_whale_std::pool_manager::SwapOperation::WhaleSwap {
                token_in_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                token_out_info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                pool_identifier: "uluna-uusd".to_string(),
            },
        ];

        // before swap uusd balance = 1_000_000_001
        // - 2*1_000 pair creation fee
        // - 1_000_000 liquidity provision
        // - 1 for native token creation (for decimal precisions)
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
                        swap_amount: Uint128::new(974)
                    })
                )
            },
        );
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

        let asset_infos = vec![
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
        ];

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
        };

        // Create a pair
        suite
            .instantiate_with_cw20_lp_token()
            .add_native_token_decimals(creator.clone(), "uwhale".to_string(), 6)
            .add_native_token_decimals(creator.clone(), "uluna".to_string(), 6)
            .create_pair(
                creator.clone(),
                asset_infos,
                pool_fees,
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                false,
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
            vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string(),
                    },
                    amount: Uint128::from(1000000u128),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    amount: Uint128::from(1000000u128),
                },
            ],
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
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::from(1000u128),
            },
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            |result| {
                println!("{:?}", result);
                *simulated_return_amount.borrow_mut() = result.unwrap().return_amount;
            },
        );

        // Now lets try a swap
        suite.swap(
            creator.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::from(1000u128),
            },
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
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
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
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
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                amount: Uint128::from(simulated_offer_amount.borrow().u128()),
            },
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
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

        let asset_infos = vec![
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
        ];

        // Default Pool fees white_whale_std::pool_network::pair::PoolFee
        // Protocol fee is 0.01% and swap fee is 0.02% and burn fee is 0%
        #[cfg(not(feature = "osmosis"))]
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
        };

        // Create a pair
        suite
            .instantiate_with_cw20_lp_token()
            .add_native_token_decimals(creator.clone(), "uwhale".to_string(), 6)
            .add_native_token_decimals(creator.clone(), "uluna".to_string(), 6)
            .create_pair(
                creator.clone(),
                asset_infos,
                pool_fees,
                white_whale_std::pool_network::asset::PairType::StableSwap { amp: 100 },
                false,
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
            vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string(),
                    },
                    amount: Uint128::from(1000000u128),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    amount: Uint128::from(1000000u128),
                },
            ],
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
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::from(1000u128),
            },
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            |result| {
                println!("{:?}", result);
                *simulated_return_amount.borrow_mut() = result.unwrap().return_amount;
            },
        );

        // Now lets try a swap
        suite.swap(
            creator.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::from(1000u128),
            },
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
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
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
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
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                amount: Uint128::from(simulated_offer_amount.borrow().u128()),
            },
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
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
        let other = suite.senders[1].clone();
        let unauthorized = suite.senders[2].clone();
        // Asset infos with uwhale and uluna

        let asset_infos = vec![
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
        ];

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
        };

        // Create a pair
        suite
            .instantiate_with_cw20_lp_token()
            .add_native_token_decimals(creator.clone(), "uwhale".to_string(), 6)
            .add_native_token_decimals(creator.clone(), "uluna".to_string(), 6)
            .create_pair(
                creator.clone(),
                asset_infos,
                pool_fees,
                white_whale_std::pool_network::asset::PairType::ConstantProduct,
                false,
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
            vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string(),
                    },
                    amount: Uint128::from(1000_000000u128),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    amount: Uint128::from(1000_000000u128),
                },
            ],
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
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::from(10000000u128),
            },
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
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

        // Verify fee collection by querying the address of the fee_collector and checking its balance
        // Should be 297 uLUNA
        suite.query_balance(
            suite.whale_lair_addr.to_string(),
            "uluna".to_string(),
            |result| {
                assert_eq!(result.unwrap().amount, Uint128::from(297u128));
            },
        );
    }
}
