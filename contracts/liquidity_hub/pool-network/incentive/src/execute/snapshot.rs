use cosmwasm_std::{DepsMut, Response, StdError, Uint128};

use crate::error::ContractError;
use crate::state::{CONFIG, GLOBAL_WEIGHT, GLOBAL_WEIGHT_SNAPSHOT};

pub fn take_global_weight_snapshot(mut deps: DepsMut) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let epoch_response: white_whale::fee_distributor::EpochResponse =
        deps.querier.query_wasm_smart(
            config.fee_distributor_address.into_string(),
            &white_whale::fee_distributor::QueryMsg::CurrentEpoch {},
        )?;

    let current_epoch = epoch_response.epoch.id.u64();

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

    GLOBAL_WEIGHT_SNAPSHOT.update::<_, StdError>(deps.storage, current_epoch, |_| {
        Ok(current_global_weight.clone())
    })?;

    Ok(Response::default().add_attributes(vec![
        ("action", "take_global_weight_snapshot".to_string()),
        ("epoch", current_epoch.to_string()),
        ("current_global_weight", current_global_weight.to_string()),
    ]))
}
