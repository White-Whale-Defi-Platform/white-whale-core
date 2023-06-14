use cosmwasm_std::{StdError, Storage};
use cw2::CONTRACT;

pub fn check_contract_name(store: &dyn Storage, new_name: String) -> Result<(), StdError> {
    let stored_contract_name = CONTRACT.load(store)?.contract;
    // Prevent accidentally migrating to a different contract
    if stored_contract_name != new_name {
        return Err(StdError::generic_err("Contract name mismatch"));
    }
    Ok(())
}
