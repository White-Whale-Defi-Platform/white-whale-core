use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::NPairInfo;
use crate::ContractError;
// use crate::tests::suite::SuiteBuilder;
use cosmwasm_std::testing::MOCK_CONTRACT_ADDR;
use cosmwasm_std::{coin, Addr, Coin, Decimal, Uint128};
use cw20::BalanceResponse;
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use white_whale::fee::Fee;
use white_whale::pool_network::asset::{Asset, AssetInfo, MINIMUM_LIQUIDITY_AMOUNT};
use white_whale::pool_network::pair::PoolFee;
use white_whale::vault_manager::LpTokenType;

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

    // Default Pool fees white_whale::pool_network::pair::PoolFee
    let fees = PoolFee {
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

    // Create a pair
    suite
        .instantiate_with_cw20_lp_token()
        .add_native_token_decimals(creator.clone(), "uwhale".to_string(), 6)
        .add_native_token_decimals(creator.clone(), "uluna".to_string(), 6)
        .create_pair(
            creator.clone(),
            asset_infos,
            fees,
            white_whale::pool_network::asset::PairType::ConstantProduct,
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

    // Default Pool fees white_whale::pool_network::pair::PoolFee
    let fees = PoolFee {
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

    // Create a pair
    suite
        .instantiate_with_cw20_lp_token()
        .add_native_token_decimals(creator.clone(), "uwhale".to_string(), 6)
        .create_pair(
            creator.clone(),
            asset_infos,
            fees,
            white_whale::pool_network::asset::PairType::ConstantProduct,
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
        suite.vault_manager_addr.clone(),
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

        // Default Pool fees white_whale::pool_network::pair::PoolFee
        let fees = PoolFee {
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

        // Create a pair
        suite
            .instantiate_with_cw20_lp_token()
            .add_native_token_decimals(creator.clone(), "uwhale".to_string(), 6)
            .create_pair(
                creator.clone(),
                asset_infos,
                fees,
                white_whale::pool_network::asset::PairType::ConstantProduct,
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

        // Default Pool fees white_whale::pool_network::pair::PoolFee
        let fees = PoolFee {
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

        // Create a pair
        suite
            .instantiate_with_cw20_lp_token()
            .add_native_token_decimals(creator.clone(), "uwhale".to_string(), 6)
            .create_pair(
                creator.clone(),
                asset_infos.clone(),
                fees.clone(),
                white_whale::pool_network::asset::PairType::ConstantProduct,
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
                fees,
                white_whale::pool_network::asset::PairType::ConstantProduct,
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

mod swapping {
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

        // Default Pool fees white_whale::pool_network::pair::PoolFee
        let fees = PoolFee {
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

        // Create a pair
        suite
            .instantiate_with_cw20_lp_token()
            .add_native_token_decimals(creator.clone(), "uwhale".to_string(), 6)
            .add_native_token_decimals(creator.clone(), "uluna".to_string(), 6)
            .create_pair(
                creator.clone(),
                asset_infos,
                fees,
                white_whale::pool_network::asset::PairType::ConstantProduct,
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
            },
        );
    }
}
