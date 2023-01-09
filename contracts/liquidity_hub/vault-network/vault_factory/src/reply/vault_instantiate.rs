use cosmwasm_std::{DepsMut, Env, Reply, Response, StdError};
use protobuf::Message;

use crate::{
    err::StdResult,
    response::MsgInstantiateContractResponse,
    state::{TMP_VAULT_ASSET, VAULTS},
};

pub fn vault_instantiate(deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
    let data = msg
        .result
        .into_result()
        .map_err(|_| StdError::GenericErr {
            msg: "Failed to get result of vault instantiation".to_string(),
        })?
        .data
        .ok_or_else(|| StdError::GenericErr {
            msg: "Failed to read binary data of vault instantiation".to_string(),
        })?;

    let res: MsgInstantiateContractResponse =
        Message::parse_from_bytes(data.as_slice()).map_err(|_| {
            StdError::parse_err(
                "MsgInstantiateContractResponse",
                "Failed to parse instantiate response",
            )
        })?;

    let vault_address = deps.api.addr_validate(&res.contract_address)?;

    // retrieve stored key from temp storage
    let (asset_info_key, asset_info) = TMP_VAULT_ASSET.load(deps.storage)?;

    // save to vault storage
    VAULTS.save(
        deps.storage,
        asset_info_key.as_slice(),
        &(vault_address.clone(), asset_info),
    )?;

    Ok(Response::new().add_attributes(vec![
        ("action", "reply_vault_instantiate"),
        ("vault_address", &vault_address.into_string()),
    ]))
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env},
        Response,
    };

    use crate::{
        contract::instantiate,
        tests::{mock_creator, mock_instantiate::mock_instantiate},
    };

    #[test]
    fn does_instantiate() {
        mock_instantiate(5, 6);
    }

    #[test]
    fn instantiate_with_response() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let creator = mock_creator();

        let res = instantiate(
            deps.as_mut(),
            env,
            creator.clone(),
            vault_network::vault_factory::InstantiateMsg {
                owner: creator.sender.into_string(),
                token_id: 5,
                vault_id: 6,
                fee_collector_addr: "fee_collector".to_string(),
            },
        )
        .unwrap();

        assert_eq!(res, Response::new())
    }
}
