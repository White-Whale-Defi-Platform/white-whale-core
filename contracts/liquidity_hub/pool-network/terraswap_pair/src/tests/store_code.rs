use cw_multi_test::{App, ContractWrapper};

use crate::contract;

/// Stores the store_pool contract to the app.
pub fn store_pool(app: &mut App) -> u64 {
    let contract = Box::new(
        ContractWrapper::new(contract::execute, contract::instantiate, contract::query)
            .with_migrate(contract::migrate)
            .with_reply(contract::reply),
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
