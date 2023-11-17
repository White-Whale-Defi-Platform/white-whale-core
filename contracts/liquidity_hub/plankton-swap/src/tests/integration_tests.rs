use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::NPairInfo;
// use crate::tests::suite::SuiteBuilder;
use cosmwasm_std::testing::MOCK_CONTRACT_ADDR;
use cosmwasm_std::{Addr, Coin, Uint128, coin};
use cw20::BalanceResponse;
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use white_whale::pool_network::asset::{Asset, AssetInfo};
use white_whale::vault_manager::LpTokenType;

use super::suite::TestingSuite;

// Using our suite lets test create pair
// and add liquidity to it

#[test]
fn instantiate_normal(){
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
