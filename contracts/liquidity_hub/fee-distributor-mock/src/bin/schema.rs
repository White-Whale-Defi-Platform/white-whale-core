use cosmwasm_schema::write_api;

use fee_distributor_mock::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

#[cfg(not(tarpaulin_include))]
fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg,
    }
}
