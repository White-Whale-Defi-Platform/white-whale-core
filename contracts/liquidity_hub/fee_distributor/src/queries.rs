use crate::state::{Config, CONFIG};
use cosmwasm_std::{Deps, StdResult};

/// Queries the [Config] of the contract
pub fn query_config(deps: Deps) -> StdResult<Config> {
    Ok(CONFIG.load(deps.storage)?)
}
