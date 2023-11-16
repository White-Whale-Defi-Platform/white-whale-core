use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::tests::suite::SuiteBuilder;
use cosmwasm_std::{Addr, Coin, Uint128};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use white_whale::pool_network::asset::{Asset, AssetInfo};

// Using our suite lets test create pair
// and add liquidity to it
#[test]
fn north_star() {
    let sender = Addr::unchecked("migaloo1xukukk68tcay629nlhnhznd9095esqln9yvc0punl645p763zd5s0tm45l");
    let mut suite = SuiteBuilder::new()
        .with_native_balances("uusd", vec![("migaloo1xukukk68tcay629nlhnhznd9095esqln9yvc0punl645p763zd5s0tm45l", 1000101), ("admin", 1000001)])
        .with_native_balances("fable", vec![("migaloo1xukukk68tcay629nlhnhznd9095esqln9yvc0punl645p763zd5s0tm45l", 1000001), ("admin", 1000001)])
        .with_cw20_balances(vec![("migaloo1xukukk68tcay629nlhnhznd9095esqln9yvc0punl645p763zd5s0tm45l", 1000000)])
        .build();

    suite
        .add_native_token_decimals(Addr::unchecked("admin"), "uusd".to_string(), 6u8)
        .unwrap();
    suite
        .add_native_token_decimals(Addr::unchecked("admin"), "fable".to_string(), 6u8)
        .unwrap();

    let asset_infos = vec![
        AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        AssetInfo::NativeToken {
            denom: "fable".to_string(),
        },
    ];

    let res = suite
        .create_constant_product_pool(sender.clone(), asset_infos, Uint128::from(100u128))
        .unwrap();
    println!("{:?}", res);

    // Lets try to add liquidity
    let res = suite
        .add_liquidity(
            sender.clone(),
            vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    amount: Uint128::from(1000000u128),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "fable".to_string(),
                    },
                    amount: Uint128::from(1000000u128),
                },
            ],
            &vec![
                Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(1000000u128),
                },
                Coin {
                    denom: "fable".to_string(),
                    amount: Uint128::from(1000000u128),
                },
            ],
            "0".to_string()
        )
        .unwrap();

    // Lets try to add liquidity
    let res = suite
        .withdraw_liquidity(
            sender.clone(),
            vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    amount: Uint128::from(1000000u128),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "fable".to_string(),
                    },
                    amount: Uint128::from(1000000u128),
                },
            ],
            &vec![
                Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(1000000u128),
                },
                Coin {
                    denom: "fable".to_string(),
                    amount: Uint128::from(1000000u128),
                },
            ],
            "0".to_string()
        )
        .unwrap();
}
