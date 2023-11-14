use cosmwasm_std::{Deps, Order, StdError, StdResult};

use white_whale::pool_network::incentive::Flow;

use crate::helpers::get_filtered_flow;
use crate::state::FLOWS;

/// Retrieves all the current flows that exist.
pub fn get_flows(
    deps: Deps,
    start_epoch: Option<u64>,
    end_epoch: Option<u64>,
) -> Result<Vec<Flow>, StdError> {
    FLOWS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<(_, Flow)>>>()?
        .into_iter()
        .map(|(_, flow)| get_filtered_flow(flow, start_epoch, end_epoch))
        .collect::<StdResult<Vec<Flow>>>()
}
