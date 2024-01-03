use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

use crate::state::{CONFIG, OPEN_POSITIONS};
use crate::ContractError;

/// Claims pending rewards for incentives where the user has LP
pub(crate) fn claim(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    cw_utils::nonpayable(&info)?;

    // check if the user has any open LP positions
    let mut open_positions = OPEN_POSITIONS
        .may_load(deps.storage, &info.sender.clone())?
        .unwrap_or(vec![]);

    if open_positions.is_empty() {
        return Err(ContractError::NoOpenPositions {});
    }

    let config = CONFIG.load(deps.storage)?;
    let current_epoch = white_whale::epoch_manager::common::get_current_epoch(
        deps.as_ref(),
        config.epoch_manager_addr.clone().into_string(),
    )?;

    Ok(Response::default().add_attributes(vec![("action", "claim".to_string())]))
}
