use crate::state::CONFIG;
use cosmwasm_std::{Deps, StdResult};
use white_whale_std::fee_distributor::Config;

/// Queries the [Config] of the contract
pub fn query_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}
