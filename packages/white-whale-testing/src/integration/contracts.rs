use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

pub fn whale_lair_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        whale_lair::contract::execute,
        whale_lair::contract::instantiate,
        whale_lair::contract::query,
    )
    .with_migrate(whale_lair::contract::migrate);

    Box::new(contract)
}

pub fn fee_distributor_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        fee_distributor::contract::execute,
        fee_distributor::contract::instantiate,
        fee_distributor::contract::query,
    )
        .with_reply(fee_distributor::contract::reply)
        .with_migrate(fee_distributor::contract::migrate);

    Box::new(contract)
}
