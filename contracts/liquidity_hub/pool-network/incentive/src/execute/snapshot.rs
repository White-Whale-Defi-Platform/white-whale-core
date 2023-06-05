use cosmwasm_std::{DepsMut, Response, StdError, Uint128};

use crate::error::ContractError;
use crate::helpers;
use crate::state::{GLOBAL_WEIGHT, GLOBAL_WEIGHT_SNAPSHOT};

/// Takes a global weight snapshot based on the current epoch
pub fn take_global_weight_snapshot(deps: DepsMut) -> Result<Response, ContractError> {
    let current_epoch = helpers::get_current_epoch(deps.as_ref())?;

    let global_weight_snapshot = GLOBAL_WEIGHT_SNAPSHOT.may_load(deps.storage, current_epoch)?;
    if global_weight_snapshot.is_some() {
        return Err(ContractError::GlobalWeightSnapshotAlreadyExists {
            epoch: current_epoch,
        });
    }

    // take the snapshot

    let current_global_weight = GLOBAL_WEIGHT
        .may_load(deps.storage)?
        .unwrap_or(Uint128::zero());

    GLOBAL_WEIGHT_SNAPSHOT
        .update::<_, StdError>(deps.storage, current_epoch, |_| Ok(current_global_weight))?;

    Ok(Response::default().add_attributes(vec![
        ("action", "take_global_weight_snapshot".to_string()),
        ("epoch", current_epoch.to_string()),
        ("current_global_weight", current_global_weight.to_string()),
    ]))
}
