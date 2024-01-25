use cosmwasm_std::{from_json, DepsMut, Reply, Response};
use protobuf::Message;

use crate::{
    error::ContractError, response::MsgInstantiateContractResponse, state::INCENTIVE_MAPPINGS,
};

/// The reply ID for submessages when creating the incentive contract from the factory.
pub const CREATE_INCENTIVE_REPLY_ID: u64 = 1;

/// Triggered after a new incentive contract is created.
///
/// Triggered to allow us to register the new contract in state.
pub fn create_incentive_reply(deps: DepsMut, msg: Reply) -> Result<Response, ContractError> {
    let res: MsgInstantiateContractResponse = Message::parse_from_bytes(
        msg.result
            .into_result()
            .map_err(|e| ContractError::CreateIncentiveCallback { reason: e })?
            .data
            .ok_or_else(|| ContractError::CreateIncentiveCallback {
                reason: String::from("Reply data was empty"),
            })?
            .as_slice(),
    )
    .map_err(|e| ContractError::CreateIncentiveCallback {
        reason: e.to_string(),
    })?;

    let incentive_address = deps.api.addr_validate(&res.address)?;

    let incentive_data: white_whale_std::pool_network::incentive::InstantiateReplyCallback =
        from_json(&res.data)?;

    INCENTIVE_MAPPINGS.save(
        deps.storage,
        incentive_data.lp_asset.to_raw(deps.api)?.as_bytes(),
        &incentive_address,
    )?;

    Ok(Response::default().add_attributes(vec![
        ("action", "create_incentive_reply".to_string()),
        ("incentive_address", incentive_address.to_string()),
        ("lp_asset", incentive_data.lp_asset.to_string()),
    ]))
}
