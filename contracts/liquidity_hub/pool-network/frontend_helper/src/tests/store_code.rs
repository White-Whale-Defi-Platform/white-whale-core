use cosmwasm_std::StdError;
use cw_multi_test::{App, ContractWrapper};

/// Stores the base CW20 contract to the app.
pub fn store_cw20_token_code(app: &mut App) -> u64 {
    let contract = Box::new(
        ContractWrapper::new(
            cw20_base::contract::execute,
            cw20_base::contract::instantiate,
            cw20_base::contract::query,
        )
        .with_migrate(cw20_base::contract::migrate),
    );

    app.store_code(contract)
}

/// Stores the incentive factory contract to the app.
pub fn store_factory_code(app: &mut App) -> u64 {
    let contract = Box::new(
        ContractWrapper::new(
            incentive_factory::contract::execute,
            incentive_factory::contract::instantiate,
            incentive_factory::contract::query,
        )
        .with_migrate(incentive_factory::contract::migrate)
        .with_reply(incentive_factory::contract::reply),
    );

    app.store_code(contract)
}

/// Stores the incentive contract to the app.
pub fn store_incentive(app: &mut App) -> u64 {
    let contract = Box::new(
        ContractWrapper::new(
            incentive::contract::execute,
            incentive::contract::instantiate,
            incentive::contract::query,
        )
        .with_migrate(incentive::contract::migrate),
    );

    app.store_code(contract)
}

/// Stores the frontend helper to the app.
pub fn store_frontend_helper(app: &mut App) -> u64 {
    let contract = Box::new(
        ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            |_, _, _: white_whale::pool_network::incentive::QueryMsg| {
                Err(StdError::generic_err(
                    "query not implemented for frontend helper",
                ))
            },
        )
        .with_reply(crate::contract::reply)
        .with_migrate(crate::contract::migrate),
    );

    app.store_code(contract)
}

pub fn store_pair(app: &mut App) -> u64 {
    let contract = Box::new(
        ContractWrapper::new(
            terraswap_pair::contract::execute,
            terraswap_pair::contract::instantiate,
            terraswap_pair::contract::query,
        )
        .with_migrate(terraswap_pair::contract::migrate)
        .with_reply(terraswap_pair::contract::reply),
    );

    app.store_code(contract)
}
