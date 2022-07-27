use cosmwasm_std::{DepsMut, Reply, Response, StdError, StdResult};
use protobuf::Message;

use crate::{response::MsgInstantiateContractResponse, state::CONFIG};

pub fn lp_instantiate(deps: DepsMut, msg: Reply) -> StdResult<Response> {
    let data = msg
        .result
        .into_result()
        .map_err(|_| StdError::GenericErr {
            msg: "Failed to get result of LP token instantiation".to_string(),
        })?
        .data
        .ok_or_else(|| StdError::GenericErr {
            msg: "Failed to read binary data of LP token instantiation".to_string(),
        })?;

    let res: MsgInstantiateContractResponse =
        Message::parse_from_bytes(data.as_slice()).map_err(|_| {
            StdError::parse_err(
                "MsgInstantiateContractResponse",
                "Failed to parse instantiate response",
            )
        })?;

    let token_address = deps.api.addr_validate(&res.contract_address)?;

    CONFIG.update::<_, StdError>(deps.storage, |mut config| {
        config.liquidity_token = token_address.clone();

        Ok(config)
    })?;

    Ok(Response::new().add_attributes(vec![
        ("action", "reply_lp_instantiate"),
        ("lp_address", &token_address.into_string()),
    ]))
}
