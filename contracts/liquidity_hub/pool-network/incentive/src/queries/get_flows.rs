use classic_bindings::TerraQuery;
use cosmwasm_std::{Deps, Order, StdError, StdResult};

use white_whale::pool_network::incentive::Flow;

use crate::state::FLOWS;

/// Retrieves all the current flows that exist.
pub fn get_flows(deps: Deps<TerraQuery>) -> Result<Vec<Flow>, StdError> {
    Ok(FLOWS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<(_, Flow)>>>()?
        .into_iter()
        .map(|(_, flow)| flow)
        .collect::<Vec<Flow>>())
}
