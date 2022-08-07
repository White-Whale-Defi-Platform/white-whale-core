use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage},
    Addr, Env, OwnedDeps,
};
use cw_multi_test::{App, Executor};
use terraswap::asset::AssetInfo;

use crate::contract::instantiate;

use super::{
    get_fees, mock_creator,
    store_code::{store_cw20_token_code, store_fee_collector_code, store_vault_code},
};

/// Instantiates the vault factory with a given `vault_id`.
pub fn mock_instantiate(
    token_id: u64,
    asset_info: AssetInfo,
) -> (OwnedDeps<MockStorage, MockApi, MockQuerier>, Env) {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let creator = mock_creator();

    instantiate(
        deps.as_mut(),
        env.clone(),
        creator.clone(),
        vault_network::vault::InstantiateMsg {
            owner: creator.sender.to_string(),
            token_id,
            asset_info,
            vault_fees: get_fees(),
            fee_collector_addr: "fee_collector".to_string(),
        },
    )
    .unwrap();

    (deps, env)
}

pub fn app_mock_instantiate(app: &mut App, asset_info: AssetInfo) -> Addr {
    let creator = mock_creator();

    let vault_id = store_vault_code(app);
    let token_id = store_cw20_token_code(app);
    let fee_collector_id = store_fee_collector_code(app);

    let fee_collector_addr = app
        .instantiate_contract(
            fee_collector_id,
            mock_creator().sender,
            &fee_collector::msg::InstantiateMsg {},
            &[],
            "mock fee collector",
            None,
        )
        .unwrap();

    app.instantiate_contract(
        vault_id,
        creator.clone().sender,
        &vault_network::vault::InstantiateMsg {
            owner: creator.sender.into_string(),
            token_id,
            asset_info,
            fee_collector_addr: fee_collector_addr.into_string(),
            vault_fees: get_fees(),
        },
        &[],
        "vault",
        None,
    )
    .unwrap()
}
