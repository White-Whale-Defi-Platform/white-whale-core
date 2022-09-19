use cosmwasm_std::{to_binary, Binary, Deps};

use crate::{err::StdResult, state::CONFIG};

/// Retrieves the contract configuration stored in state.
pub fn get_config(deps: Deps) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;

    Ok(to_binary(&config)?)
}
