use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

/// Creates the incentive manager contract
pub fn incentive_manager_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        incentive_manager::contract::execute,
        incentive_manager::contract::instantiate,
        incentive_manager::contract::query,
    )
    .with_migrate(incentive_manager::contract::migrate);

    Box::new(contract)
}

/// Creates the whale lair contract
pub fn whale_lair_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        whale_lair::contract::execute,
        whale_lair::contract::instantiate,
        whale_lair::contract::query,
    )
    .with_migrate(whale_lair::contract::migrate);

    Box::new(contract)
}

/// Creates the epoch manager contract
pub fn epoch_manager_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        epoch_manager::contract::execute,
        epoch_manager::contract::instantiate,
        epoch_manager::contract::query,
    )
    .with_migrate(epoch_manager::contract::migrate);

    Box::new(contract)
}

/// Creates a pair contract
pub fn pair_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        terraswap_pair::contract::execute,
        terraswap_pair::contract::instantiate,
        terraswap_pair::contract::query,
    )
    .with_reply(terraswap_pair::contract::reply)
    .with_migrate(terraswap_pair::contract::migrate);

    Box::new(contract)
}
