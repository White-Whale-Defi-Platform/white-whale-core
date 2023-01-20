use cosmwasm_std::{Deps, StdResult};
use crate::state::{CONFIG, Config};

/// Queries the [Config] of the contract
pub fn query_config(deps: Deps) -> StdResult<Config> {
    Ok(CONFIG.load(deps.storage)?)
}
