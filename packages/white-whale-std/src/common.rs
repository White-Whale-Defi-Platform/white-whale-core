use cosmwasm_std::{Addr, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Storage};
use cw_ownable::{Action, OwnershipError};
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

/// Updates the ownership of a contract using the cw_ownable package, which needs to be implemented by the contract.
pub fn update_ownership(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    action: Action,
) -> Result<Response, OwnershipError> {
    cw_ownable::update_ownership(deps, &env.block, &info.sender, action).map(|ownership| {
        Response::default()
            .add_attribute("action", "update_ownership")
            .add_attributes(ownership.into_attributes())
    })
}

/// Validates a [String] address or returns the default address if the validation fails.
pub fn validate_addr_or_default(deps: &Deps, unvalidated: Option<String>, default: Addr) -> Addr {
    unvalidated
        .map_or_else(
            || Some(default.clone()),
            |recv| match deps.api.addr_validate(&recv) {
                Ok(validated) => Some(validated),
                Err(_) => None,
            },
        )
        .unwrap_or(default)
}
