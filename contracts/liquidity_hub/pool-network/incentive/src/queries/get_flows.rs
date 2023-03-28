use cosmwasm_std::{Deps, StdError};
use white_whale::pool_network::incentive::Flow;

use crate::state::FLOWS;

/// Retrieves all the current flows that exist.
pub fn get_flows(deps: Deps) -> Result<Vec<Flow>, StdError> {
    FLOWS.load(deps.storage)
}
