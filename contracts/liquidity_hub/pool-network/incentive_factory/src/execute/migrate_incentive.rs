use cosmwasm_std::{to_binary, DepsMut, MessageInfo, Response, WasmMsg};

use crate::{error::ContractError, state::CONFIG};

pub fn migrate_incentive(
    deps: DepsMut,
    info: MessageInfo,
    incentive_address: String,
    code_id: Option<u64>,
) -> Result<Response, ContractError> {
    // check that sender has permissions to perform migration
    let config = CONFIG.load(deps.storage)?;
    if info.sender != deps.api.addr_humanize(&config.owner)? {
        return Err(ContractError::Unauthorized);
    }

    // if `code_id` was unspecified, we default to the config incentive code id
    let new_code_id = code_id.unwrap_or(config.incentive_code_id);

    Ok(Response::new().add_message(WasmMsg::Migrate {
        contract_addr: incentive_address,
        new_code_id,
        msg: to_binary(&pool_network::incentive::MigrateMsg {})?,
    }))
}
