use cosmwasm_std::Empty;
use cw_multi_test::{App, Contract, ContractWrapper};

pub fn whale_lair_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        whale_lair::contract::execute,
        whale_lair::contract::instantiate,
        whale_lair::contract::query,
    )
    .with_migrate(whale_lair::contract::migrate);

    Box::new(contract)
}

/// Stores the fee distributor contract to the app
pub fn store_fee_distributor_code(app: &mut App) -> u64 {
    let contract = Box::new(
        ContractWrapper::new_with_empty(
            fee_distributor::contract::execute,
            fee_distributor::contract::instantiate,
            fee_distributor::contract::query,
        )
        .with_reply(fee_distributor::contract::reply)
        .with_migrate(fee_distributor::contract::migrate),
    );

    app.store_code(contract)
}

/// Stores the fee collector contract to the app
pub fn store_fee_collector_code(app: &mut App) -> u64 {
    let contract = Box::new(
        ContractWrapper::new_with_empty(
            fee_collector::contract::execute,
            fee_collector::contract::instantiate,
            fee_collector::contract::query,
        )
        .with_migrate(fee_collector::contract::migrate)
        .with_reply(fee_collector::contract::reply),
    );

    app.store_code(contract)
}
