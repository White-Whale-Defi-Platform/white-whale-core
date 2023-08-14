use classic_bindings::TerraQuery;
use cosmwasm_std::{
    coins, to_binary, BankMsg, CosmosMsg, DepsMut, MessageInfo, Order, Response, StdResult, WasmMsg,
};
use white_whale::pool_network::asset::AssetInfo;
use white_whale::pool_network::incentive::Flow;

use crate::{
    error::ContractError,
    state::{CONFIG, FLOWS},
};

/// Closes the flow with the given id and return the unclaimed assets to the flow creator
pub fn close_flow(
    deps: DepsMut<TerraQuery>,
    info: MessageInfo,
    flow_id: u64,
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
        .find(|(_, flow)| flow.flow_id == flow_id)
        .ok_or(ContractError::NonExistentFlow {
            invalid_id: flow_id,
        })
        .map(|(_, flow)| flow)?;

    if !(flow.flow_creator == info.sender || info.sender == factory_config.owner) {
        return Err(ContractError::UnauthorizedFlowClose { flow_id });
    }

    let amount_to_return = flow.flow_asset.amount.saturating_sub(flow.claimed_amount);

    // return the flow assets available, i.e. the ones that haven't been claimed
    let messages: Vec<CosmosMsg> = vec![match flow.flow_asset.info {
        AssetInfo::NativeToken { denom } => BankMsg::Send {
            to_address: flow.flow_creator.clone().into_string(),
            amount: coins(amount_to_return.u128(), denom),
        }
        .into(),
        AssetInfo::Token { contract_addr } => WasmMsg::Execute {
            contract_addr,
            msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                recipient: flow.flow_creator.clone().into_string(),
                amount: amount_to_return,
            })?,
            funds: vec![],
        }
        .into(),
    }];

    // close the flow by removing it from storage
    FLOWS.remove(deps.storage, (flow.start_epoch, flow.flow_id));

    Ok(Response::default()
        .add_attributes(vec![
            ("action", "close_flow".to_string()),
            ("flow_id", flow_id.to_string()),
        ])
        .add_messages(messages))
}
