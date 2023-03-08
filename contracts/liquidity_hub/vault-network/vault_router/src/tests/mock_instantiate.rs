use cosmwasm_std::{
    coins,
    testing::{mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage},
    Addr, Env, OwnedDeps, Uint128,
};
use cw_multi_test::{App, Executor};
use pool_network::asset::AssetInfo;

use crate::contract::instantiate;

use super::{
    get_fees, mock_admin, mock_creator,
    store_code::{
        store_cw20_token_code, store_factory_code, store_fee_collector_code, store_router_code,
        store_vault_code,
    },
};

/// Instantiates the vault router with a given vault factory address.
pub fn mock_instantiate<F: Into<String>>(
    vault_factory_addr: F,
) -> (OwnedDeps<MockStorage, MockApi, MockQuerier>, Env) {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let creator = mock_creator();

    instantiate(
        deps.as_mut(),
        env.clone(),
        creator.clone(),
        vault_network::vault_router::InstantiateMsg {
            owner: creator.sender.to_string(),
            vault_factory_addr: vault_factory_addr.into(),
        },
    )
    .unwrap();

    (deps, env)
}

pub struct AppInstantiateResponse {
    pub router_addr: Addr,
    pub token_addr: Addr,
    pub native_vault_addr: Addr,
    pub token_vault_addr: Addr,
    pub factory_addr: Addr,
}

pub fn app_mock_instantiate(app: &mut App) -> AppInstantiateResponse {
    let creator = mock_creator();

    let factory_id = store_factory_code(app);
    let vault_id = store_vault_code(app);
    let token_id = store_cw20_token_code(app);
    let fee_collector_id = store_fee_collector_code(app);
    let router_id = store_router_code(app);

    let fee_collector_addr = app
        .instantiate_contract(
            fee_collector_id,
            mock_admin(),
            &fee_collector::msg::InstantiateMsg {},
            &[],
            "mock fee collector",
            None,
        )
        .unwrap();

    let factory_addr = app
        .instantiate_contract(
            factory_id,
            mock_admin(),
            &vault_network::vault_factory::InstantiateMsg {
                owner: mock_admin().into_string(),
                vault_id,
                token_id,
                fee_collector_addr: fee_collector_addr.into_string(),
            },
            &[],
            "mock vault factory",
            None,
        )
        .unwrap();

    // create two vaults
    app.execute_contract(
        mock_admin(),
        factory_addr.clone(),
        &vault_network::vault_factory::ExecuteMsg::CreateVault {
            asset_info: AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            fees: get_fees(),
        },
        &[],
    )
    .unwrap();
    // deposit to native vault
    let native_vault_addr: Addr = app
        .wrap()
        .query_wasm_smart(
            factory_addr.clone(),
            &vault_network::vault_factory::QueryMsg::Vault {
                asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
            },
        )
        .unwrap();
    app.send_tokens(
        mock_admin(),
        native_vault_addr.clone(),
        &coins(10_000, "uluna"),
    )
    .unwrap();

    // first create token with 10k supply to admin
    let token_addr = app
        .instantiate_contract(
            token_id,
            mock_admin(),
            &cw20_base::msg::InstantiateMsg {
                decimals: 6,
                initial_balances: vec![cw20::Cw20Coin {
                    address: mock_admin().into_string(),
                    // give the admin a 5k buffer of token to use
                    amount: Uint128::new(15_000),
                }],
                marketing: None,
                mint: None,
                name: "mock cw20 vault token".to_string(),
                symbol: "cwV".to_string(),
            },
            &[],
            "mock cw20 vault token",
            None,
        )
        .unwrap();

    app.execute_contract(
        mock_admin(),
        factory_addr.clone(),
        &vault_network::vault_factory::ExecuteMsg::CreateVault {
            asset_info: AssetInfo::Token {
                contract_addr: token_addr.clone().into_string(),
            },
            fees: get_fees(),
        },
        &[],
    )
    .unwrap();
    let token_vault_addr: Addr = app
        .wrap()
        .query_wasm_smart(
            factory_addr.clone(),
            &vault_network::vault_factory::QueryMsg::Vault {
                asset_info: AssetInfo::Token {
                    contract_addr: token_addr.clone().into_string(),
                },
            },
        )
        .unwrap();

    // deposit all the token funds into the vault
    app.execute_contract(
        mock_admin(),
        token_addr.clone(),
        &cw20::Cw20ExecuteMsg::IncreaseAllowance {
            spender: token_vault_addr.clone().into_string(),
            amount: Uint128::new(10_000),
            expires: None,
        },
        &[],
    )
    .unwrap();
    app.execute_contract(
        mock_admin(),
        token_vault_addr.clone(),
        &vault_network::vault::ExecuteMsg::Deposit {
            amount: Uint128::new(10_000),
        },
        &[],
    )
    .unwrap();

    let router_addr = app
        .instantiate_contract(
            router_id,
            creator.clone().sender,
            &vault_network::vault_router::InstantiateMsg {
                owner: creator.sender.into_string(),
                vault_factory_addr: factory_addr.clone().into_string(),
            },
            &[],
            "mock vault router",
            None,
        )
        .unwrap();

    AppInstantiateResponse {
        router_addr,
        token_addr,
        native_vault_addr,
        token_vault_addr,
        factory_addr,
    }
}
