use classic_bindings::TerraQuery;
use cosmwasm_std::{Addr, Decimal256, Deps, Uint128};

use white_whale_std::pool_network::incentive::RewardsShareResponse;

use crate::error::ContractError;
use crate::helpers;
use crate::state::{EpochId, ADDRESS_WEIGHT_HISTORY, GLOBAL_WEIGHT_SNAPSHOT};

#[allow(unused_assignments)]
/// Gets the global weight for the current epoch. Returns a [GlobalWeightResponse] struct or an error
/// if no global weight snapshot has been taken for the current epoch.
pub fn get_rewards_share(
    deps: Deps<TerraQuery>,
    address: Addr,
) -> Result<RewardsShareResponse, ContractError> {
    let mut last_epoch_user_weight_update: EpochId = 0u64;
    let mut last_user_weight_seen: Uint128 = Uint128::zero();
    let earliest_available_weight_for_user =
        helpers::get_earliest_available_weight_snapshot_for_user(deps, &&address)?;

    let current_epoch = helpers::get_current_epoch(deps)?;

    if !earliest_available_weight_for_user.is_empty() {
        (last_epoch_user_weight_update, last_user_weight_seen) =
            earliest_available_weight_for_user[0];
    } else {
        let global_weight_at_current_epoch = GLOBAL_WEIGHT_SNAPSHOT
            .may_load(deps.storage, current_epoch)?
            .unwrap_or_default();

        return Ok(RewardsShareResponse {
            address,
            global_weight: global_weight_at_current_epoch,
            address_weight: Uint128::zero(),
            share: Decimal256::zero(),
            epoch_id: current_epoch,
        });
    }

    let start_epoch = last_epoch_user_weight_update;
    for epoch_id in start_epoch..=current_epoch {
        let user_weight_at_epoch =
            ADDRESS_WEIGHT_HISTORY.may_load(deps.storage, (&address.clone(), epoch_id))?;

        if let Some(user_weight_at_epoch) = user_weight_at_epoch {
            last_user_weight_seen = user_weight_at_epoch;
        }
    }

    let global_weight_at_current_epoch =
        GLOBAL_WEIGHT_SNAPSHOT.may_load(deps.storage, current_epoch)?;

    if let Some(global_weight_snapshot) = global_weight_at_current_epoch {
        let user_share_at_epoch =
            Decimal256::from_ratio(last_user_weight_seen, global_weight_snapshot);

        Ok(RewardsShareResponse {
            address,
            global_weight: global_weight_snapshot,
            address_weight: last_user_weight_seen,
            share: user_share_at_epoch,
            epoch_id: current_epoch,
        })
    } else {
        Err(ContractError::GlobalWeightSnapshotNotTakenForEpoch {
            epoch: current_epoch,
        })
    }
}
