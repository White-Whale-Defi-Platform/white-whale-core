use cosmwasm_schema::write_api;

use white_whale_std::epoch_manager::epoch_manager::{ExecuteMsg, InstantiateMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg,
    }
}
