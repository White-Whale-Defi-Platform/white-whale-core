use cosmwasm_std::{Deps, Order, StdError, StdResult};

use white_whale::pool_network::incentive::{Flow, FlowResponse};

use crate::state::FLOWS;

pub fn get_flow(deps: Deps, flow_id: u64) -> Result<Option<FlowResponse>, StdError> {
    Ok(FLOWS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<((_, _), Flow)>>>()?
        .into_iter()
        .find(|((_, _), flow)| flow.flow_id == flow_id)
        .map(|((_, _), flow)| FlowResponse { flow: Some(flow) }))

    // todo remove
    // Ok(FLOWS
    //     .load(deps.storage)?
    //     .into_iter()
    //     .find(|flow| flow.flow_id == flow_id)
    //     .map(|flow| FlowResponse { flow: Some(flow) }))
}
