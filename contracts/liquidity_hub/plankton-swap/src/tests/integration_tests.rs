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
        MOCK_CONTRACT_ADDR.to_string(),
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
        MOCK_CONTRACT_ADDR.to_string(),
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
            |result| println!("{:?}", result),
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

// #[test]
// fn north_star() {
//     let sender =
//         Addr::unchecked("migaloo1xukukk68tcay629nlhnhznd9095esqln9yvc0punl645p763zd5s0tm45l");
//     let mut suite = SuiteBuilder::new()
//         .with_native_balances(
//             "uusd",
//             vec![
//                 (
//                     "migaloo1xukukk68tcay629nlhnhznd9095esqln9yvc0punl645p763zd5s0tm45l",
//                     1000101,
//                 ),
//                 ("admin", 1000001),
//             ],
//         )
//         .with_native_balances(
//             "fable",
//             vec![
//                 (
//                     "migaloo1xukukk68tcay629nlhnhznd9095esqln9yvc0punl645p763zd5s0tm45l",
//                     1000001,
//                 ),
//                 ("admin", 1000001),
//             ],
//         )
//         .with_cw20_balances(vec![(
//             "migaloo1xukukk68tcay629nlhnhznd9095esqln9yvc0punl645p763zd5s0tm45l",
//             1000000,
//         )])
//         .build();

//     suite
//         .add_native_token_decimals(Addr::unchecked("admin"), "uusd".to_string(), 6u8)
//         .unwrap();
//     suite
//         .add_native_token_decimals(Addr::unchecked("admin"), "fable".to_string(), 6u8)
//         .unwrap();

//     let asset_infos = vec![
//         AssetInfo::NativeToken {
//             denom: "uusd".to_string(),
//         },
//         AssetInfo::NativeToken {
//             denom: "fable".to_string(),
//         },
//     ];

//     let res = suite
//         .create_constant_product_pool(sender.clone(), asset_infos, Uint128::from(100u128))
//         .unwrap();
//     println!("{:?}", res);

//     // Lets try to add liquidity
//     let res = suite
//         .add_liquidity(
//             sender.clone(),
//             vec![
//                 Asset {
//                     info: AssetInfo::NativeToken {
//                         denom: "uusd".to_string(),
//                     },
//                     amount: Uint128::from(1000000u128),
//                 },
//                 Asset {
//                     info: AssetInfo::NativeToken {
//                         denom: "fable".to_string(),
//                     },
//                     amount: Uint128::from(1000000u128),
//                 },
//             ],
//             &vec![
//                 Coin {
//                     denom: "uusd".to_string(),
//                     amount: Uint128::from(1000000u128),
//                 },
//                 Coin {
//                     denom: "fable".to_string(),
//                     amount: Uint128::from(1000000u128),
//                 },
//             ],
//             "0".to_string(),
//         )
//         .unwrap();

//     // Get the token from config
//     let pair_resp: NPairInfo = suite
//         .app
//         .wrap()
//         .query_wasm_smart(
//             suite.pool_manager_addr.clone(),
//             &crate::msg::QueryMsg::Pair {
//                 pair_identifier: "0".to_string(),
//             },
//         )
//         .unwrap();

//     // Now get balance we have the address
//     let lp_token_addr = match pair_resp.liquidity_token {
//         AssetInfo::Token { contract_addr } => contract_addr,
//         _ => {
//             panic!("Liquidity token is not a cw20 token")
//         }
//     };

//     let lp_token_balance: BalanceResponse = suite
//         .app
//         .wrap()
//         .query_wasm_smart(
//             lp_token_addr,
//             &cw20::Cw20QueryMsg::Balance {
//                 address: sender.to_string(),
//             },
//         )
//         .unwrap();

//     println!("{:?}", lp_token_balance);

//     // Lets try to add liquidity
//     let res = suite
//         .withdraw_liquidity_cw20(
//             sender.clone(),
//             vec![
//                 Asset {
//                     info: AssetInfo::NativeToken {
//                         denom: "uusd".to_string(),
//                     },
//                     amount: Uint128::from(1000000u128),
//                 },
//                 Asset {
//                     info: AssetInfo::NativeToken {
//                         denom: "fable".to_string(),
//                     },
//                     amount: Uint128::from(1000000u128),
//                 },
//             ],
//             "0".to_string(),
//             lp_token_balance.balance,
//         )
//         .unwrap();
// }
