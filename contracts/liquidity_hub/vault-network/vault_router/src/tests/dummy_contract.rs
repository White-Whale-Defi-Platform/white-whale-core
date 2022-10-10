use cosmwasm_std::{Addr, BankMsg, Coin, Response};
use cw_multi_test::{App, ContractWrapper, Executor};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{mock_admin, store_code::store_dummy_flash_loan_contract};

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Send { to_address: Addr, amount: Vec<Coin> },
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct InstantiateMsg {}
#[derive(Debug, Deserialize, Clone)]
pub enum QueryMsg {}

#[derive(Error, Debug)]
pub enum DummyError {}

/// Creates a new dummy flash loan for use in tests.
pub fn create_dummy_flash_loan_contract(
) -> ContractWrapper<ExecuteMsg, InstantiateMsg, QueryMsg, DummyError, DummyError, DummyError> {
    ContractWrapper::new(
        |_deps, _env, _info, msg| match msg {
            ExecuteMsg::Send { to_address, amount } => {
                Ok(Response::new().add_message(BankMsg::Send {
                    to_address: to_address.to_string(),
                    amount,
                }))
            }
        },
        |_deps, _env, _info, _msg| Ok(Response::new()),
        |_deps, _env, _query| unimplemented!(),
    )
}

/// Uploads and instantiates the dummy contract, returning the address of the contract.
pub fn create_dummy_contract(app: &mut App) -> Addr {
    let code_id = store_dummy_flash_loan_contract(app);

    app.instantiate_contract(
        code_id,
        mock_admin(),
        &InstantiateMsg {},
        &[],
        "dummy flash-loan",
        None,
    )
    .unwrap()
}
