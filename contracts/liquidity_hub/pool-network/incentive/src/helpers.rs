use classic_bindings::TerraQuery;
use cosmwasm_std::{Addr, Deps, DepsMut, Order, StdError, StdResult, Uint128};

use white_whale_std::pool_network::incentive::Flow;

use crate::error::ContractError;
use crate::state::{EpochId, ADDRESS_WEIGHT_HISTORY, CONFIG, FLOWS};

/// Gets the current epoch from the fee distributor contract.
pub fn get_current_epoch(deps: Deps<TerraQuery>) -> Result<u64, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let epoch_response: white_whale_std::fee_distributor::EpochResponse =
        deps.querier.query_wasm_smart(
            config.fee_distributor_address.into_string(),
            &white_whale_std::fee_distributor::QueryMsg::CurrentEpoch {},
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

/// Gets the flow asset amount for a given epoch, taking into account the asset history, i.e. flow expansion.
pub fn get_flow_asset_amount_at_epoch(flow: &Flow, epoch: u64) -> Uint128 {
    let mut asset_amount = flow.flow_asset.amount;

    if let Some((_, &(change_amount, _))) = flow.asset_history.range(..=epoch).rev().next() {
        asset_amount = change_amount;
    }

    asset_amount
}

/// Gets the flow end_epoch, taking into account flow expansion.
pub fn get_flow_end_epoch(flow: &Flow) -> u64 {
    let mut end_epoch = flow.end_epoch;

    if let Some((_, &(_, expanded_end_epoch))) = flow.asset_history.last_key_value() {
        end_epoch = expanded_end_epoch;
    }

    end_epoch
}

/// Gets the flow end_epoch, taking into account flow expansion.
pub fn get_flow_current_end_epoch(flow: &Flow, epoch: u64) -> u64 {
    let mut end_epoch = flow.end_epoch;

    if let Some((_, &(_, current_end_epoch))) = flow.asset_history.range(..=epoch).rev().next() {
        end_epoch = current_end_epoch;
    }

    end_epoch
}

pub const MAX_EPOCH_LIMIT: u64 = 100;

/// Gets a [Flow] filtering the asset history and emitted tokens to the given range of epochs.
pub fn get_filtered_flow(
    mut flow: Flow,
    start_epoch: Option<u64>,
    end_epoch: Option<u64>,
) -> StdResult<Flow> {
    let start_range = start_epoch.unwrap_or(flow.start_epoch);
    let mut end_range = end_epoch.unwrap_or(
        start_range
            .checked_add(MAX_EPOCH_LIMIT)
            .ok_or_else(|| StdError::generic_err("Overflow"))?,
    );

    if end_range.saturating_sub(start_range) > MAX_EPOCH_LIMIT {
        end_range = start_range
            .checked_add(MAX_EPOCH_LIMIT)
            .ok_or_else(|| StdError::generic_err("Overflow"))?;
    }

    flow.asset_history
        .retain(|&k, _| k >= start_range && k <= end_range);
    flow.emitted_tokens
        .retain(|k, _| *k >= start_range && *k <= end_range);

    Ok(flow)
}
