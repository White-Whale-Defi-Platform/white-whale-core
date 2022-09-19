use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage},
    Addr, Env, OwnedDeps,
};
use cw_multi_test::{App, Executor};

use crate::contract::instantiate;

use super::{
    mock_creator,
    store_code::{
        store_cw20_token_code, store_factory_code, store_fee_collector_code, store_router_code,
        store_vault_code,
    },
};

/// Instantiates the vault router with a given vault factory address.
pub fn mock_instantiate(
    vault_factory_addr: String,
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
            vault_factory_addr,
        },
    )
    .unwrap();

    (deps, env)
}

pub fn app_mock_instantiate(app: &mut App) -> Addr {
    let creator = mock_creator();

    let factory_id = store_factory_code(app);
    let vault_id = store_vault_code(app);
    let token_id = store_cw20_token_code(app);
    let fee_collector_id = store_fee_collector_code(app);
    let router_id = store_router_code(app);

    let fee_collector_addr = app
        .instantiate_contract(
            fee_collector_id,
            creator.sender.clone(),
            &fee_collector::msg::InstantiateMsg {},
            &[],
            "mock fee collector",
            None,
        )
        .unwrap();

    let factory_addr = app
        .instantiate_contract(
            factory_id,
            creator.clone().sender,
            &vault_network::vault_factory::InstantiateMsg {
                owner: creator.sender.clone().into_string(),
                vault_id,
                token_id,
                fee_collector_addr: fee_collector_addr.into_string(),
            },
            &[],
            "vault_factory",
            None,
        )
        .unwrap();

    app.instantiate_contract(
        router_id,
        creator.clone().sender,
        &vault_network::vault_router::InstantiateMsg {
            owner: creator.sender.into_string(),
            vault_factory_addr: factory_addr.into_string(),
        },
        &[],
        "vault_router",
        None,
    )
    .unwrap()
}
