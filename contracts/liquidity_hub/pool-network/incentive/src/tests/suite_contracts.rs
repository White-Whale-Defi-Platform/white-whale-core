use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

use white_whale::pool_network::incentive;

pub fn incentive_factory_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        incentive_factory::contract::execute,
        incentive_factory::contract::instantiate,
        incentive_factory::contract::query,
    )
    .with_reply(incentive_factory::contract::reply)
    .with_migrate(incentive_factory::contract::migrate);

    Box::new(contract)
}

pub fn incentive_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_migrate(crate::contract::migrate);

    Box::new(contract)
}

pub fn fee_collector_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        fee_collector::contract::execute,
        fee_collector::contract::instantiate,
        fee_collector::contract::query,
    )
    .with_reply(fee_collector::contract::reply)
    .with_migrate(fee_collector::contract::migrate);

    Box::new(contract)
}

pub fn cw20_token_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        terraswap_token::contract::execute,
        terraswap_token::contract::instantiate,
        terraswap_token::contract::query,
    );

    Box::new(contract)
}
