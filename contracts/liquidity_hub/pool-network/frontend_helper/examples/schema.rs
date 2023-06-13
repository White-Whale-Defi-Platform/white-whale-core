use cosmwasm_schema::write_api;

use white_whale::pool_network::frontend_helper::{ExecuteMsg, InstantiateMsg, MigrateMsg};

fn main() {
    write_api! {
        name: "frontend-helper",
        version: "1.0.0",
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        migrate: MigrateMsg,
    }
}
