use cosmwasm_std::{Addr, StdError, StdResult, Storage};
use cw_storage_plus::Item;

/// Validates that the given address matches the address stored in the given `owner_item`.
pub fn validate_owner(
    storage: &dyn Storage,
    owner_item: Item<Addr>,
    address: Addr,
) -> StdResult<()> {
    let owner = owner_item.load(storage)?;

    // verify owner
    if owner != address {
        return Err(StdError::generic_err("Unauthorized"));
    }

    Ok(())
}
