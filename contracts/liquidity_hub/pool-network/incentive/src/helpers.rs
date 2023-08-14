use classic_bindings::TerraQuery;
use cosmwasm_std::{Addr, Deps, DepsMut, Order, StdResult, Uint128};

use white_whale::pool_network::incentive::Flow;

use crate::error::ContractError;
use crate::state::{EpochId, ADDRESS_WEIGHT_HISTORY, CONFIG, FLOWS};

/// Gets the current epoch from the fee distributor contract.
pub fn get_current_epoch(deps: Deps<TerraQuery>) -> Result<u64, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let epoch_response: white_whale::fee_distributor::EpochResponse =
        deps.querier.query_wasm_smart(
            config.fee_distributor_address.into_string(),
            &white_whale::fee_distributor::QueryMsg::CurrentEpoch {},
        )?;

    Ok(epoch_response.epoch.id.u64())
}

/// Gets the flows that are available for the current epoch, i.e. those flows that started either on
/// the epoch provided or before it.
pub fn get_available_flows(
    deps: Deps<TerraQuery>,
    epoch: &u64,
) -> Result<Vec<Flow>, ContractError> {
    Ok(FLOWS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?
        .into_iter()
        .filter(|((start_epoch, _), _)| start_epoch <= epoch)
        .map(|(_, flow)| flow)
        .collect::<Vec<Flow>>())
}

/// Gets the earliest available weight snapshot recorded for the user.
pub fn get_earliest_available_weight_snapshot_for_user(
    deps: Deps<TerraQuery>,
    address: &&Addr,
) -> Result<Vec<(EpochId, Uint128)>, ContractError> {
    Ok(ADDRESS_WEIGHT_HISTORY
        .prefix(address)
        .range(deps.storage, None, None, Order::Ascending)
        .take(1) // take only one item, the first item. Since it's being sorted in ascending order, it's the earliest one.
        .collect::<StdResult<Vec<(EpochId, Uint128)>>>()?)
}

// Deletes all the weight history entries for the given user
pub fn delete_weight_history_for_user(
    deps: &mut DepsMut<TerraQuery>,
    address: &&Addr,
) -> Result<(), ContractError> {
    let address_weight_history_epoch_keys_for_sender = ADDRESS_WEIGHT_HISTORY
        .prefix(&(*address).clone())
        .keys(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<u64>>>()?;

    address_weight_history_epoch_keys_for_sender
        .iter()
        .for_each(|&epoch_key| {
            ADDRESS_WEIGHT_HISTORY.remove(deps.storage, (address, epoch_key));
        });
    Ok(())
}
