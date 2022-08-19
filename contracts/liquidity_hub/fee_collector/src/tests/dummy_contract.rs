use cosmwasm_std::{to_binary, Addr, BankMsg, Coin, Response};
use cw_multi_test::ContractWrapper;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Send { to_address: Addr, amount: Vec<Coin> },
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct InstantiateMsg {}
#[derive(Debug, Deserialize, Clone)]
pub enum QueryMsg {}

#[derive(Error, Debug)]
pub enum VaultError {}

pub fn create_dummy_flash_loan_contract(
) -> ContractWrapper<ExecuteMsg, InstantiateMsg, QueryMsg, VaultError, VaultError, VaultError> {
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
        |_deps, _env, _query| Ok(to_binary::<Vec<Coin>>(&vec![]).unwrap()),
    )
}
