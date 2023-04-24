use cosmwasm_std::{Addr, Coin};
use cw_multi_test::{App, AppBuilder, BankKeeper};

pub fn mock_app() -> App {
    App::default()
}

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
