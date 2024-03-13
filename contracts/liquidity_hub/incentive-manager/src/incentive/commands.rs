use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

use crate::state::{get_open_positions_by_receiver, CONFIG};
use crate::ContractError;

/// Claims pending rewards for incentives where the user has LP
pub(crate) fn claim(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    cw_utils::nonpayable(&info)?;

    // check if the user has any open LP positions
    let open_positions = get_open_positions_by_receiver(deps.storage, info.sender.into_string())?;

    if open_positions.is_empty() {
        return Err(ContractError::NoOpenPositions);
    }

    let config = CONFIG.load(deps.storage)?;
    let _current_epoch = white_whale_std::epoch_manager::common::get_current_epoch(
        deps.as_ref(),
        config.epoch_manager_addr.into_string(),
    )?;

    //todo complete this

    Ok(Response::default().add_attributes(vec![("action", "claim".to_string())]))
}
