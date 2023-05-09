use cosmwasm_std::{Deps, StdError};
use white_whale::pool_network::incentive::FlowResponse;

use crate::state::FLOWS;

pub fn get_flow(deps: Deps, flow_id: u64) -> Result<Option<FlowResponse>, StdError> {
    Ok(FLOWS
        .load(deps.storage)?
        .into_iter()
        .find(|flow| flow.flow_id == flow_id)
        .map(|flow| FlowResponse { flow: Some(flow) }))
}
