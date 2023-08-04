use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::tests::suite::SuiteBuilder;
use cosmwasm_std::{Addr, Uint128};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use white_whale::pool_network::asset::{AssetInfo, Asset};

// Using our suite lets test create pair
// and add liquidity to it
#[test]
fn north_star() {
    let mut app = App::default();

    let mut suite = SuiteBuilder::new()
        .with_native_balances("uusd", vec![("addr0000", 1000000)])
        .with_cw20_balances(vec![("addr0000", 1000000)])
        .build();

    let asset_infos = vec![
        AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        AssetInfo::Token {
            contract_addr: "addr0000".to_string(),
        },
    ];
    let res = suite
        .create_constant_product_pool(Addr::unchecked("addr0000"), asset_infos)
        .unwrap();
    println!("{:?}", res);
    assert_eq!(1,0);

    // Lets try to add liquidity
    let res = suite
        .add_liquidity(
            Addr::unchecked("addr0000"),
            vec![Asset{
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(1000000u128),
            },
            Asset{
                info: AssetInfo::Token {
                    contract_addr: "addr0000".to_string(),
                },
                amount: Uint128::from(1000000u128),
            }],
        ).unwrap();
}
