use classic_bindings::TerraQuery;
use cosmwasm_std::{Deps, Order, StdError, StdResult};

use white_whale_std::pool_network::incentive::{Flow, FlowIdentifier, FlowResponse};

use crate::helpers::get_filtered_flow;
use crate::state::FLOWS;

/// Gets a flow given the [FlowIdentifier].
pub fn get_flow(
    deps: Deps<TerraQuery>,
    flow_identifier: FlowIdentifier,
    start_epoch: Option<u64>,
    end_epoch: Option<u64>,
) -> Result<Option<FlowResponse>, StdError> {
    FLOWS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<(_, Flow)>>>()?
        .into_iter()
        .find(|(_, flow)| match &flow_identifier {
            FlowIdentifier::Id(id) => flow.flow_id == *id,
            FlowIdentifier::Label(label) => flow.flow_label.as_ref() == Some(label),
        })
        .map(|(_, flow)| {
            get_filtered_flow(flow, start_epoch, end_epoch).map(|filtered_flow| FlowResponse {
                flow: Some(filtered_flow),
            })
        })
        .transpose()
}
