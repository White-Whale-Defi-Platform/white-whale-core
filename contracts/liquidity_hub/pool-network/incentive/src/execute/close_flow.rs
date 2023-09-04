use classic_bindings::TerraQuery;

use white_whale::pool_network::asset::Asset;
use cosmwasm_std::{
    coins, to_binary, BankMsg, CosmosMsg, DepsMut, MessageInfo, Order, Response, StdResult, WasmMsg,
};

use white_whale::pool_network::asset::AssetInfo;
use white_whale::pool_network::incentive::{Flow, FlowIdentifier};

use crate::{
    error::ContractError,
    state::{CONFIG, FLOWS},
};

/// Closes the flow with the given id and return the unclaimed assets to the flow creator
pub fn close_flow(
    deps: DepsMut<TerraQuery>,
    info: MessageInfo,
    flow_identifier: FlowIdentifier,
) -> Result<Response, ContractError> {
    // validate that user is allowed to close the flow
    let config = CONFIG.load(deps.storage)?;
    let factory_config: white_whale::pool_network::incentive_factory::ConfigResponse =
        deps.querier.query_wasm_smart(
            config.factory_address.into_string(),
            &white_whale::pool_network::incentive_factory::QueryMsg::Config {},
        )?;

    let flow = FLOWS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<(_, Flow)>>>()?
        .into_iter()
        .find(|(_, flow)| match &flow_identifier.clone() {
            FlowIdentifier::Id(id) => flow.flow_id == *id,
            FlowIdentifier::Label(label) => flow.flow_label.as_ref() == Some(label),
        })
        .ok_or(ContractError::NonExistentFlow {
            invalid_identifier: flow_identifier.clone(),
        })
        .map(|(_, flow)| flow)?;

    if !(flow.flow_creator == info.sender || info.sender == factory_config.owner) {
        return Err(ContractError::UnauthorizedFlowClose { flow_identifier });
    }

    // return the flow assets available, i.e. the ones that haven't been claimed
    let amount_to_return = flow.flow_asset.amount.saturating_sub(flow.claimed_amount);
    let refund_msg = Asset {
        info: flow.flow_asset.info,
        amount: amount_to_return,
    }
    .into_msg(&deps.querier, flow.flow_creator)?;

    // close the flow by removing it from storage
    FLOWS.remove(deps.storage, (flow.start_epoch, flow.flow_id));

    Ok(Response::default()
        .add_attributes(vec![
            ("action", "close_flow".to_string()),
            ("flow_identifier", flow_identifier.to_string()),
        ])
        .add_message(refund_msg))
}
