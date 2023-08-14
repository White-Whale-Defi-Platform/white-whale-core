use classic_bindings::TerraQuery;
use cosmwasm_std::{Deps, StdError};
use white_whale::pool_network::incentive::ConfigResponse;

use crate::state::CONFIG;

/// Retrieves the configuration of the contract.
pub fn get_config(deps: Deps<TerraQuery>) -> Result<ConfigResponse, StdError> {
    CONFIG.load(deps.storage)
}
