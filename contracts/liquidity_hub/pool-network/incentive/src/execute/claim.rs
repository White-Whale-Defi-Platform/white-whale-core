use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdError};

use crate::error::ContractError;
use crate::state::{CONFIG, GLOBAL_WEIGHT, GLOBAL_WEIGHT_SNAPSHOT, LAST_CLAIMED_EPOCH};

pub fn claim(mut deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    // Ok(Response::default()
    //     .add_attributes(vec![("action", "claim")])
    //     .add_messages(crate::claim::claim(&mut deps, &env, &info)?))

    // TODO new stuff, remove/refactor old stuff

    // check what's the last global weight epoch, and snapshot it if it's not already done
    let config = CONFIG.load(deps.storage)?;
    let epoch_response: white_whale::fee_distributor::EpochResponse =
        deps.querier.query_wasm_smart(
            config.fee_distributor_address.into_string(),
            &white_whale::fee_distributor::QueryMsg::CurrentEpoch {},
        )?;

    let current_epoch = epoch_response.epoch.id.u64();

    let global_weight_snapshot = GLOBAL_WEIGHT_SNAPSHOT.may_load(deps.storage, current_epoch)?;

    if global_weight_snapshot.is_none() {
        return Err(ContractError::GlobalWeightSnapshotNotTaken {});
    }

    Ok(Response::default()
        .add_attributes(vec![("action", "claim")])
        .add_messages(crate::claim::claim2(&mut deps, &env, &info)?))
}
