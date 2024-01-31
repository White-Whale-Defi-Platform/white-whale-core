use crate::state::CONFIG;
use classic_bindings::TerraQuery;
use cosmwasm_std::{Deps, StdResult};
use white_whale_std::fee_distributor::Config;

/// Queries the [Config] of the contract
pub fn query_config(deps: Deps<TerraQuery>) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}
