use cosmwasm_std::{to_json_binary, DepsMut, Env, ReplyOn, Response, SubMsg, WasmMsg};
use white_whale_std::pool_network::asset::AssetInfo;

use crate::{
    error::ContractError,
    reply::create_incentive_reply::CREATE_INCENTIVE_REPLY_ID,
    state::{CONFIG, INCENTIVE_MAPPINGS},
};

/// Creates a new incentive contract.
pub fn create_incentive(
    deps: DepsMut,
    env: Env,
    lp_asset: AssetInfo,
) -> Result<Response, ContractError> {
    // ensure that lp_asset doesn't already have an incentive contract
    if INCENTIVE_MAPPINGS.has(deps.storage, lp_asset.to_raw(deps.api)?.as_bytes()) {
        return Err(ContractError::DuplicateIncentiveContract {
            incentive: lp_asset,
        });
    }

    // create the incentive
    let config = CONFIG.load(deps.storage)?;

    // a LP token label is in the format of {label}-{label}-LP
    // where `label` is a length of 3-12 characters
    // this means we have a max length of 28 characters for the label
    // this fits within the limits of the 128 MaxLabelSize defined in wasm
    Ok(Response::default()
        .add_attributes(vec![
            ("action", "create_incentive".to_string()),
            ("lp_asset", lp_asset.to_string()),
        ])
        .add_submessage(SubMsg {
            id: CREATE_INCENTIVE_REPLY_ID,
            gas_limit: None,
            reply_on: ReplyOn::Always,
            msg: WasmMsg::Instantiate {
                admin: Some(env.contract.address.into_string()),
                code_id: config.incentive_code_id,
                msg: to_json_binary(&white_whale_std::pool_network::incentive::InstantiateMsg {
                    lp_asset: lp_asset.clone(),
                    fee_distributor_address: config.fee_distributor_addr.into_string(),
                })?,
                funds: vec![],
                label: format!("{} incentives", lp_asset.get_label(&deps.as_ref())?),
            }
            .into(),
        }))
}
