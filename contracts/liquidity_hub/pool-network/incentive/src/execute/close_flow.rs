use cosmwasm_std::{
    coins, to_binary, BankMsg, CosmosMsg, DepsMut, MessageInfo, Response, StdError, WasmMsg,
};
use white_whale::pool_network::asset::AssetInfo;

use crate::{
    error::ContractError,
    state::{CONFIG, FLOWS},
};

pub fn close_flow(
    deps: DepsMut,
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

    let flow = {
        let flows = FLOWS.load(deps.storage)?;

        // search for the `flow_id` specified
        flows
            .into_iter()
            .find(|flow| flow.flow_id == flow_id)
            .ok_or(ContractError::NonExistentFlow {
                invalid_id: flow_id,
            })?
    };

    if !(flow.flow_creator == info.sender || info.sender == factory_config.owner) {
        return Err(ContractError::UnauthorizedFlowClose { flow_id });
    }

    // return the flow assets available
    let messages: Vec<CosmosMsg> = vec![match flow.flow_asset.info {
        AssetInfo::NativeToken { denom } => BankMsg::Send {
            to_address: flow.flow_creator.clone().into_string(),
            amount: coins(flow.flow_asset.amount.u128(), denom),
        }
        .into(),
        AssetInfo::Token { contract_addr } => WasmMsg::Execute {
            contract_addr,
            msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                recipient: flow.flow_creator.clone().into_string(),
                amount: flow.flow_asset.amount,
            })?,
            funds: vec![],
        }
        .into(),
    }];

    // close the flow by removing it from storage
    FLOWS.update::<_, StdError>(deps.storage, |flows| {
        Ok(flows
            .into_iter()
            .filter(|flow| flow.flow_id != flow_id)
            .collect())
    })?;

    Ok(Response::default()
        // .add_attributes(vec![
        //     ("action", "close_flow".to_string()),
        //     ("flow_id", flow_id.to_string()),
        // ])
        .add_messages(messages))
}
