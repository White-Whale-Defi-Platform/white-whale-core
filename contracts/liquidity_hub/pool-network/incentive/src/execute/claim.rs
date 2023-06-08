use cosmwasm_std::{DepsMut, MessageInfo, Response};

use crate::error::ContractError;
use crate::helpers;
use crate::state::GLOBAL_WEIGHT_SNAPSHOT;

/// Claim available rewards for the user.
pub fn claim(mut deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    // check what's the last global weight epoch, and snapshot it if it's not already done
    let current_epoch = helpers::get_current_epoch(deps.as_ref())?;

    let global_weight_snapshot = GLOBAL_WEIGHT_SNAPSHOT.may_load(deps.storage, current_epoch)?;
    if global_weight_snapshot.is_none() {
        return Err(ContractError::GlobalWeightSnapshotNotTakenForEpoch {
            epoch: current_epoch,
        });
    }

    Ok(Response::default()
        .add_attributes(vec![("action", "claim")])
        .add_messages(crate::claim::claim(&mut deps, &info)?))
}
