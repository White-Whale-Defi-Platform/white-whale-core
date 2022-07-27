use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage},
    Addr, Env, OwnedDeps,
};
use cw_multi_test::{App, Executor};

use crate::contract::instantiate;

use super::mock_creator;

/// Instantiates the vault factory with a given `vault_id`.
pub fn mock_instantiate(
    vault_id: u64,
    token_id: u64,
) -> (OwnedDeps<MockStorage, MockApi, MockQuerier>, Env) {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let creator = mock_creator();

    instantiate(
        deps.as_mut(),
        env.clone(),
        creator.clone(),
        vault_network::vault_factory::InstantiateMsg {
            owner: creator.sender.to_string(),
            vault_id,
            token_id,
        },
    )
    .unwrap();

    (deps, env)
}

pub fn app_mock_instantiate(app: &mut App, factory_id: u64, vault_id: u64, token_id: u64) -> Addr {
    let creator = mock_creator();

    app.instantiate_contract(
        factory_id,
        creator.clone().sender,
        &vault_network::vault_factory::InstantiateMsg {
            owner: creator.sender.into_string(),
            vault_id,
            token_id,
        },
        &[],
        "vault_factory",
        None,
    )
    .unwrap()
}
