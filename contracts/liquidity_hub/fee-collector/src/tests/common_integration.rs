use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{Addr, Coin, MessageInfo, Uint128};
use cw_multi_test::{App, AppBuilder, BankKeeper, ContractWrapper, Executor};

use crate::contract::{execute, instantiate, migrate, query};

/// Mocks the App
pub fn mock_app() -> App {
    App::default()
}

/// Mocks the App with balance
pub fn mock_app_with_balance(balances: Vec<(Addr, Vec<Coin>)>) -> App {
    let bank = BankKeeper::new();

    AppBuilder::new()
        .with_bank(bank)
        .build(|router, _api, storage| {
            balances.into_iter().for_each(|(account, amount)| {
                router.bank.init_balance(storage, &account, amount).unwrap()
            });
        })
}

/// Creates a mock creator
pub fn mock_creator() -> MessageInfo {
    mock_info("creator", &[])
}

/// Stores the pool factory contract to the app
pub fn store_pool_factory_code(app: &mut App) -> u64 {
    let contract = Box::new(
        ContractWrapper::new_with_empty(
            terraswap_factory::contract::execute,
            terraswap_factory::contract::instantiate,
            terraswap_factory::contract::query,
        )
        .with_reply(terraswap_factory::contract::reply)
        .with_migrate(terraswap_factory::contract::migrate),
    );

    app.store_code(contract)
}

/// Stores the fee collector contract to the app
pub fn store_fee_collector_code(app: &mut App) -> u64 {
    let contract = Box::new(
        ContractWrapper::new_with_empty(execute, instantiate, query).with_migrate(migrate),
    );

    app.store_code(contract)
}

/// Stores the pair contract to the app
pub fn store_pair_code(app: &mut App) -> u64 {
    let contract = Box::new(
        ContractWrapper::new_with_empty(
            terraswap_pair::contract::execute,
            terraswap_pair::contract::instantiate,
            terraswap_pair::contract::query,
        )
        .with_reply(terraswap_pair::contract::reply)
        .with_migrate(terraswap_pair::contract::migrate),
    );

    app.store_code(contract)
}

/// Stores the token contract to the app
pub fn store_token_code(app: &mut App) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        terraswap_token::contract::execute,
        terraswap_token::contract::instantiate,
        terraswap_token::contract::query,
    ));

    app.store_code(contract)
}

pub fn increase_allowance(app: &mut App, sender: Addr, contract_addr: Addr, spender: Addr) {
    app.execute_contract(
        sender,
        contract_addr,
        &cw20::Cw20ExecuteMsg::IncreaseAllowance {
            spender: spender.to_string(),
            amount: Uint128::new(500_000_000_000u128),
            expires: None,
        },
        &[],
    )
    .unwrap();
}
