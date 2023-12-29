use cosmwasm_std::{Deps, StdResult};

use crate::epoch_manager::epoch_manager::{EpochResponse, QueryMsg};

/// Queries the current epoch from the epoch manager contract
pub fn get_current_epoch(deps: Deps, epoch_manager_addr: String) -> StdResult<u64> {
    let epoch_response: EpochResponse = deps
        .querier
        .query_wasm_smart(epoch_manager_addr, &QueryMsg::CurrentEpoch {})?;

    Ok(epoch_response.epoch.id)
}
