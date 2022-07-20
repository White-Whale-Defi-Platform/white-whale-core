use cw_multi_test::{App, ContractWrapper};

use crate::{
    contract::{execute, instantiate, migrate, query},
    reply::reply,
};

/// Stores the vault factory contract to the app.
pub fn store_factory_code(app: &mut App) -> u64 {
    let contract = Box::new(
        ContractWrapper::new_with_empty(execute, instantiate, query)
            .with_reply(reply)
            .with_migrate(migrate),
    );

    app.store_code(contract)
}

/// Stores the base CW20 contract to the app.
pub fn store_cw20_token_code(app: &mut App) -> u64 {
    let contract = Box::new(ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    ));

    app.store_code(contract)
}

pub fn store_vault_code(app: &mut App) -> u64 {
    let contract = Box::new(
        ContractWrapper::new(
            vault::contract::execute,
            vault::contract::instantiate,
            vault::contract::query,
        )
        .with_reply(vault::reply::reply),
    );

    app.store_code(contract)
}
