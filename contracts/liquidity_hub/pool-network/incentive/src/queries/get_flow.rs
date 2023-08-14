use classic_bindings::TerraQuery;
use cosmwasm_std::{Deps, Order, StdError, StdResult};

use white_whale::pool_network::incentive::{Flow, FlowResponse};

use crate::state::FLOWS;

pub fn get_flow(deps: Deps<TerraQuery>, flow_id: u64) -> Result<Option<FlowResponse>, StdError> {
    Ok(FLOWS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<(_, Flow)>>>()?
        .into_iter()
        .find(|(_, flow)| flow.flow_id == flow_id)
        .map(|(_, flow)| FlowResponse { flow: Some(flow) }))
}
