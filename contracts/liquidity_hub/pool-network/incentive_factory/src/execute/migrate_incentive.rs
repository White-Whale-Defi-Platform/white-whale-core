use classic_bindings::TerraQuery;
use cosmwasm_std::{to_binary, DepsMut, MessageInfo, Response, WasmMsg};

use crate::{error::ContractError, state::CONFIG};

pub fn migrate_incentive(
    deps: DepsMut<TerraQuery>,
    info: MessageInfo,
    incentive_address: String,
    code_id: Option<u64>,
) -> Result<Response, ContractError> {
    // check that sender has permissions to perform migration
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized);
    }

    // if `code_id` was unspecified, we default to the config incentive code id
    let new_code_id = code_id.unwrap_or(config.incentive_code_id);

    Ok(Response::default()
        .add_attributes(vec![
            ("action", "migrate_incentive".to_string()),
            ("incentive_address", incentive_address.clone()),
            ("new_code_id", new_code_id.to_string()),
        ])
        .add_message(WasmMsg::Migrate {
            contract_addr: incentive_address,
            new_code_id,
            msg: to_binary(&white_whale::pool_network::incentive::MigrateMsg {})?,
        }))
}
