use crate::state::{CONFIG, LAST_CLAIMED_EPOCH};
use crate::ContractError;
use cosmwasm_std::{Addr, Deps};
use white_whale_std::epoch_manager::common::get_current_epoch;
use white_whale_std::incentive_manager::RewardsResponse;

pub(crate) fn _get_rewards(deps: Deps, address: Addr) -> Result<RewardsResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let current_epoch = get_current_epoch(deps, config.epoch_manager_addr.into_string())?;

    let last_claimed_epoch = LAST_CLAIMED_EPOCH.may_load(deps.storage, &address)?;

    // Check if the user ever claimed before
    if let Some(last_claimed_epoch) = last_claimed_epoch {
        // if the last claimed epoch is the same as the current epoch, then there is nothing to claim
        if current_epoch.id == last_claimed_epoch {
            return Ok(RewardsResponse::RewardsResponse { rewards: vec![] });
        }
    }

    let rewards = vec![];

    Ok(RewardsResponse::RewardsResponse { rewards })
}