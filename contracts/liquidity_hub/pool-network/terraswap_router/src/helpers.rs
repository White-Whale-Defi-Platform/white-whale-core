use cosmwasm_std::{Addr, Deps, Env};
use cw_storage_plus::Path;
use white_whale_std::pool_network::router::{SwapOperation, SwapRoute};

use crate::{error::ContractError, state::SWAP_ROUTES};

/// This function compares the address of the message sender with the contract admin
/// address. This provides a convenient way to verify if the sender
/// is the admin in a single line.
pub fn assert_admin(deps: Deps, env: &Env, sender: &Addr) -> Result<(), ContractError> {
    let contract_info = deps
        .querier
        .query_wasm_contract_info(env.contract.address.clone())?;
    if let Some(admin) = contract_info.admin {
        if sender != deps.api.addr_validate(admin.as_str())? {
            return Err(ContractError::Unauthorized {});
        }
    }
    Ok(())
}

/// This function returns the key for a given swap route by computing the offer
/// and ask asset labels.
pub fn get_key_from_swap_route(
    deps: Deps,
    swap_route: &SwapRoute,
) -> Result<Path<Vec<SwapOperation>>, ContractError> {
    Ok(SWAP_ROUTES.key((
        swap_route
            .clone()
            .offer_asset_info
            .get_label(&deps)?
            .as_str(),
        swap_route.clone().ask_asset_info.get_label(&deps)?.as_str(),
    )))
}
