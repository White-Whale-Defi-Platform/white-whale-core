pub mod contract;
pub mod execute;
mod migrations;
pub mod queries;
pub mod reply;
pub mod response;
pub mod state;

pub mod err;

#[cfg(test)]
pub mod tests;

#[cfg(test)]
mod asdasdas {
    use cosmwasm_std::{CosmosMsg, WasmMsg};

    #[test]
    fn test() {
        println!(
            "{:?}",
            CosmosMsg::<String>::Wasm(WasmMsg::UpdateAdmin {
                contract_addr: "contract".to_string(),
                admin: "admin".to_string()
            })
        );
    }
}
