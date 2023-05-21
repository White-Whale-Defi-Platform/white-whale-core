use cosmwasm_schema::write_api;

use white_whale::fee_collector::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

fn main() {
    write_api! {
        name: "fee_collector",
        version: "1.0.2",
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
        migrate: MigrateMsg,
    }
}
