use cosmwasm_std::{Deps, StdError, StdResult, Timestamp};

use crate::constants::DAY_SECONDS;
use crate::epoch_manager::epoch_manager::{EpochResponse, EpochV2, QueryMsg};

/// Queries the current epoch from the epoch manager contract
pub fn get_current_epoch(deps: Deps, epoch_manager_addr: String) -> StdResult<EpochV2> {
    let epoch_response: EpochResponse = deps
        .querier
        .query_wasm_smart(epoch_manager_addr, &QueryMsg::CurrentEpoch {})?;

    Ok(epoch_response.epoch)
}

/// Validates that the given epoch has not expired, i.e. not more than 24 hours have passed since the start of the epoch.
pub fn validate_epoch(epoch: &EpochV2, current_time: Timestamp) -> StdResult<()> {
    if current_time
        .minus_seconds(epoch.start_time.seconds())
        .seconds()
        < DAY_SECONDS
    {
        return Err(StdError::generic_err(
            "Current epoch has expired, please wait for the next epoch to start.",
        ));
    }

    Ok(())
}
