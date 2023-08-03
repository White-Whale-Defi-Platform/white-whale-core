use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::tests::suite::SuiteBuilder;
use cosmwasm_std::Addr;
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use white_whale::pool_network::asset::AssetInfo;

// Using our suite lets test create pair
// and add liquidity to it
#[test]
fn test_create_pair_and_add_liquidity() {
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
}
