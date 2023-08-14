use classic_bindings::TerraQuery;
use cosmwasm_std::Deps;

use white_whale::pool_network::incentive::GlobalWeightResponse;

use crate::error::ContractError;
use crate::state::GLOBAL_WEIGHT_SNAPSHOT;

/// Gets the global weight for the given epoch. Returns a [GlobalWeightResponse] struct or an error
/// if no global weight snapshot has been taken for that epoch.
pub fn get_global_weight(
    deps: Deps<TerraQuery>,
    epoch_id: u64,
) -> Result<GlobalWeightResponse, ContractError> {
    let global_weight_snapshot = GLOBAL_WEIGHT_SNAPSHOT.may_load(deps.storage, epoch_id)?;

    if let Some(global_weight_snapshot) = global_weight_snapshot {
        Ok(GlobalWeightResponse {
            global_weight: global_weight_snapshot,
            epoch_id,
        })
    } else {
        Err(ContractError::GlobalWeightSnapshotNotTakenForEpoch { epoch: epoch_id })
    }
}
