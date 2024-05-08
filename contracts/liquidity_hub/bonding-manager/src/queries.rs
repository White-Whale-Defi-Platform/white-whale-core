use std::collections::{HashSet, VecDeque};
use white_whale_std::epoch_manager::epoch_manager::ConfigResponse;

use cosmwasm_std::{
    to_json_binary, Decimal, Deps, Order, QueryRequest, StdError, StdResult, Timestamp, Uint128,
    Uint64, WasmQuery,
};
use cw_storage_plus::Bound;

use white_whale_std::bonding_manager::{
    Bond, BondedResponse, BondingWeightResponse, Config, GlobalIndex, UnbondingResponse,
    WithdrawableResponse,
};
use white_whale_std::bonding_manager::{ClaimableEpochsResponse, Epoch};
use white_whale_std::epoch_manager::epoch_manager::QueryMsg;

use crate::helpers;
use crate::state::{
    get_weight, BOND, BONDING_ASSETS_LIMIT, CONFIG, EPOCHS, GLOBAL, LAST_CLAIMED_EPOCH, UNBOND,
};

/// Queries the current configuration of the contract.
pub(crate) fn query_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

/// Queries the current bonded amount of the given address. If no address is provided, returns
/// the global bonded amount.
pub(crate) fn query_bonded(deps: Deps, address: Option<String>) -> StdResult<BondedResponse> {
    let (total_bonded, bonded_assets, first_bonded_epoch_id) = if let Some(address) = address {
        let address = deps.api.addr_validate(&address)?;

        let bonds: Vec<Bond> = BOND
            .prefix(&address)
            .range(deps.storage, None, None, Order::Ascending)
            .take(BONDING_ASSETS_LIMIT)
            .map(|item| {
                let (_, bond) = item?;
                Ok(bond)
            })
            .collect::<StdResult<Vec<Bond>>>()?;

        // if it doesn't have bonded, return empty response
        if bonds.is_empty() {
            return Ok(BondedResponse {
                total_bonded: Uint128::zero(),
                bonded_assets: vec![],
                first_bonded_epoch_id: Some(Uint64::zero()),
            });
        }

        let mut total_bonded = Uint128::zero();
        let mut bonded_assets = vec![];

        // 1 January 2500
        let mut first_bond_timestamp = Timestamp::from_seconds(16725229261u64);

        for bond in bonds {
            if bond.timestamp.seconds() < first_bond_timestamp.seconds() {
                first_bond_timestamp = bond.timestamp;
            }

            total_bonded = total_bonded.checked_add(bond.asset.amount)?;
            bonded_assets.push(bond.asset);
        }

        let config = CONFIG.load(deps.storage)?;
        // Query epoch manager for EpochConfig
        let epoch_config: ConfigResponse =
            deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: config.epoch_manager_addr.to_string(),
                msg: to_json_binary(&QueryMsg::Config {})?,
            }))?;

        let first_bonded_epoch_id =
            helpers::calculate_epoch(epoch_config.epoch_config, first_bond_timestamp)?;

        (total_bonded, bonded_assets, Some(first_bonded_epoch_id))
    } else {
        let global_index = GLOBAL.may_load(deps.storage)?.unwrap_or_default();
        (global_index.bonded_amount, global_index.bonded_assets, None)
    };

    Ok(BondedResponse {
        total_bonded,
        bonded_assets,
        first_bonded_epoch_id,
    })
}

pub const MAX_PAGE_LIMIT: u8 = 30u8;
pub const DEFAULT_PAGE_LIMIT: u8 = 10u8;

/// Queries the current unbonding amount of the given address.
pub(crate) fn query_unbonding(
    deps: Deps,
    address: String,
    denom: String,
    start_after: Option<u64>,
    limit: Option<u8>,
) -> StdResult<UnbondingResponse> {
    let address = deps.api.addr_validate(&address)?;
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT).min(MAX_PAGE_LIMIT) as usize;
    let start = calc_range_start(start_after).map(Bound::ExclusiveRaw);

    let unbonding = UNBOND
        .prefix((&deps.api.addr_validate(address.as_str())?, &denom))
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, bond) = item?;
            Ok(bond)
        })
        .collect::<StdResult<Vec<Bond>>>()?;
    // aggregate all the amounts in unbonding vec and return uint128
    let unbonding_amount = unbonding.iter().try_fold(Uint128::zero(), |acc, bond| {
        acc.checked_add(bond.asset.amount)
    })?;

    Ok(UnbondingResponse {
        total_amount: unbonding_amount,
        unbonding_requests: unbonding,
    })
}

fn calc_range_start(start_after: Option<u64>) -> Option<Vec<u8>> {
    start_after.map(|block_height| {
        let mut v: Vec<u8> = block_height.to_be_bytes().to_vec();
        v.push(0);
        v
    })
}

/// Queries the amount of unbonding tokens of the specified address that have passed the
/// unbonding period and can be withdrawn.
pub(crate) fn query_withdrawable(
    deps: Deps,
    timestamp: Timestamp,
    address: String,
    denom: String,
) -> StdResult<WithdrawableResponse> {
    let config = CONFIG.load(deps.storage)?;
    let unbonding: StdResult<Vec<_>> = UNBOND
        .prefix((&deps.api.addr_validate(address.as_str())?, &denom))
        .range(deps.storage, None, None, Order::Ascending)
        .take(MAX_PAGE_LIMIT as usize)
        .collect();

    let mut withdrawable_amount = Uint128::zero();
    for (_, bond) in unbonding? {
        if timestamp.minus_nanos(config.unbonding_period.u64()) >= bond.timestamp {
            withdrawable_amount = withdrawable_amount.checked_add(bond.asset.amount)?;
        }
    }

    Ok(WithdrawableResponse {
        withdrawable_amount,
    })
}

/// Queries the current weight of the given address.
pub(crate) fn query_weight(
    deps: Deps,
    timestamp: Timestamp,
    address: String,
    global_index: Option<GlobalIndex>,
) -> StdResult<BondingWeightResponse> {
    let address = deps.api.addr_validate(&address)?;

    let bonds: StdResult<Vec<_>> = BOND
        .prefix(&address)
        .range(deps.storage, None, None, Order::Ascending)
        .take(MAX_PAGE_LIMIT as usize)
        .collect();

    let config = CONFIG.load(deps.storage)?;

    let mut total_bond_weight = Uint128::zero();
    // Search bonds for unique bond.asset.denoms
    // Make an empty set of unique denoms
    let mut unique_denoms: HashSet<String> = HashSet::new();

    for (_, mut bond) in bonds? {
        bond.weight = get_weight(
            timestamp,
            bond.weight,
            bond.asset.amount,
            config.growth_rate,
            bond.timestamp,
        )?;

        if !unique_denoms.contains(&bond.asset.denom) {
            unique_denoms.insert(bond.asset.denom.clone());
        }
        // Aggregate the weights of all the bonds for the given address.
        // This assumes bonding assets are fungible.
        total_bond_weight = total_bond_weight.checked_add(bond.weight)?;
    }

    // If a global weight from an Epoch was passed, use that to get the weight, otherwise use the current global index weight
    let mut global_index = if let Some(global_index) = global_index {
        global_index
    } else {
        GLOBAL
            .may_load(deps.storage)
            .unwrap_or_else(|_| Some(GlobalIndex::default()))
            .ok_or_else(|| StdError::generic_err("Global index not found"))?
    };

    global_index.weight = get_weight(
        timestamp,
        global_index.weight,
        global_index.bonded_amount,
        config.growth_rate,
        global_index.timestamp,
    )?;

    // Represents the share of the global weight that the address has
    // If global_index.weight is zero no one has bonded yet so the share is
    let share = if global_index.weight.is_zero() {
        Decimal::zero()
    } else {
        Decimal::from_ratio(total_bond_weight, global_index.weight)
    };

    Ok(BondingWeightResponse {
        address: address.to_string(),
        weight: total_bond_weight,
        global_weight: global_index.weight,
        share,
        timestamp,
    })
}

/// Queries the global index
pub fn query_global_index(deps: Deps) -> StdResult<GlobalIndex> {
    let global_index = GLOBAL.may_load(deps.storage)?.unwrap_or_default();
    Ok(global_index)
}

/// Returns the epoch that is falling out the grace period, which is the one expiring after creating
/// a new epoch is created.
pub fn get_expiring_epoch(deps: Deps) -> StdResult<Option<Epoch>> {
    let config = CONFIG.load(deps.storage)?;
    // Adding 1 because we store the future epoch in the map also, so grace_period + 1
    let grace_period_plus_future_epoch = config.grace_period.u64() + 1u64;
    // Take grace_period + 1 and then slice last one off
    let epochs = EPOCHS
        .range(deps.storage, None, None, Order::Descending)
        .take(grace_period_plus_future_epoch as usize)
        .map(|item| {
            let (_, epoch) = item?;
            Ok(epoch)
        })
        .collect::<StdResult<Vec<Epoch>>>()?;

    // if the epochs vector's length is the same as the grace period it means there is one epoch that
    // is expiring once the new one is created i.e. the last epoch in the vector
    if epochs.len() == grace_period_plus_future_epoch as usize {
        let expiring_epoch: Epoch = epochs.into_iter().last().unwrap_or_default();
        Ok(Some(expiring_epoch))
    } else {
        // nothing is expiring yet
        Ok(None)
    }
}

/// Returns the epochs that are within the grace period, i.e. the ones which fees can still be claimed.
/// The result is ordered by epoch id, descending. Thus, the first element is the current epoch.
pub fn get_claimable_epochs(deps: Deps) -> StdResult<ClaimableEpochsResponse> {
    let config = CONFIG.load(deps.storage)?;
    // Adding 1 because we store the future epoch in the map also, so grace_period + 1
    let grace_period = config.grace_period.u64() + 1;
    // Take grace_period + 1 and then slice last one off
    let mut epochs = EPOCHS
        .range(deps.storage, None, None, Order::Descending)
        .take(grace_period as usize)
        .map(|item| {
            let (_, epoch) = item?;

            Ok(epoch)
        })
        .collect::<StdResult<VecDeque<Epoch>>>()?;

    println!("epochs: {:?}", epochs);

    // Remove the upcoming epoch from stack
    epochs.pop_front();
    epochs.retain(|epoch| !epoch.available.is_empty());

    println!("epochs: {:?}", epochs);

    Ok(ClaimableEpochsResponse {
        epochs: epochs.into(),
    })
}

/// Returns the epochs that can be claimed by the given address. If no address is provided,
/// returns all possible epochs stored in the contract that can potentially be claimed.
pub fn query_claimable(deps: Deps, address: Option<String>) -> StdResult<ClaimableEpochsResponse> {
    let mut claimable_epochs = get_claimable_epochs(deps)?.epochs;

    // if an address is provided, filter what's claimable for that address
    if let Some(address) = address {
        let address = deps.api.addr_validate(&address)?;

        let last_claimed_epoch = LAST_CLAIMED_EPOCH.may_load(deps.storage, &address)?;

        // filter out epochs that have already been claimed by the user
        if let Some(last_claimed_epoch) = last_claimed_epoch {
            claimable_epochs.retain(|epoch| epoch.id > last_claimed_epoch);
        } else {
            // if the user doesn't have any last_claimed_epoch two things might be happening:
            // 1- the user has never bonded before
            // 2- the user has bonded, but never claimed any rewards so far

            let bonded_response: BondedResponse = query_bonded(deps, Some(address.into_string()))?;
            println!("bonded_responsebonded_response: {:?}", bonded_response);
            if bonded_response.bonded_assets.is_empty() {
                // the user has never bonded before, therefore it shouldn't be able to claim anything
                claimable_epochs.clear();
            } else {
                // the user has bonded, but never claimed any rewards so far. The first_bonded_epoch_id
                // value should always be Some here, as `query_bonded` will always return Some.
                match bonded_response.first_bonded_epoch_id {
                    Some(first_bonded_epoch_id) => {
                        // keep all epochs that are newer than the first bonded epoch
                        claimable_epochs.retain(|epoch| epoch.id > first_bonded_epoch_id);
                    }
                    None => {
                        // for sanity, it should never happen
                        claimable_epochs.clear();
                    }
                }
            }
        };
        // filter out epochs that have no available fees. This would only happen in case the grace period
        // gets increased after epochs have expired, which would lead to make them available for claiming
        // again without any available rewards, as those were forwarded to newer epochs.
        claimable_epochs.retain(|epoch| !epoch.available.is_empty());
    }

    Ok(ClaimableEpochsResponse {
        epochs: claimable_epochs,
    })
}
