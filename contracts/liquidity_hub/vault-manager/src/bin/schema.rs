use cosmwasm_schema::write_api;

use white_whale::vault_manager::{ExecuteMsg, InstantiateMsg, ManagerConfig, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg,
    }
}
