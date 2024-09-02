use cosmwasm_std::{to_json_binary, Addr, Deps, Order, QueryRequest, StdResult, Uint64, WasmQuery};
use cw_storage_plus::{Item, Map};

use white_whale_std::fee_distributor::{ClaimableEpochsResponse, Config, Epoch, EpochResponse};
use white_whale_std::whale_lair::{BondedResponse, QueryMsg};

pub const CONFIG: Item<Config> = Item::new("config");
pub const LAST_CLAIMED_EPOCH: Map<&Addr, Uint64> = Map::new("last_claimed_epoch");
pub const EPOCHS: Map<&[u8], Epoch> = Map::new("epochs");

/// Returns the current epoch, which is the last on the EPOCHS map.
pub fn get_current_epoch(deps: Deps) -> StdResult<EpochResponse> {
    let option = EPOCHS
        .range(deps.storage, None, None, Order::Descending)
        .next();

    let epoch = match option {
        Some(Ok((_, epoch))) => epoch,
        _ => Epoch::default(),
    };

    Ok(EpochResponse { epoch })
}

/// Returns the [Epoch] with the given id.
pub fn get_epoch(deps: Deps, id: Uint64) -> StdResult<EpochResponse> {
    let epoch = EPOCHS
        .may_load(deps.storage, &id.to_be_bytes())?
        .unwrap_or_default();

    Ok(EpochResponse { epoch })
}

/// Returns the epoch that is falling out the grace period, which is the one expiring after creating
/// a new epoch is created.
pub fn get_expiring_epoch(deps: Deps) -> StdResult<Option<Epoch>> {
    let grace_period = CONFIG.load(deps.storage)?.grace_period;

    // last epochs within the grace period
    let epochs = EPOCHS
        .range(deps.storage, None, None, Order::Descending)
        .take(grace_period.u64() as usize)
        .map(|item| {
            let (_, epoch) = item?;
            Ok(epoch)
        })
        .collect::<StdResult<Vec<Epoch>>>()?;

    // if the epochs vector's length is the same as the grace period it means there is one epoch that
    // is expiring once the new one is created i.e. the last epoch in the vector
    if epochs.len() == grace_period.u64() as usize {
        Ok(Some(epochs.last().cloned().unwrap_or_default()))
    } else {
        // nothing is expiring yet
        Ok(None)
    }
}

/// Returns the epochs that are within the grace period, i.e. the ones which fees can still be claimed.
/// The result is ordered by epoch id, descending. Thus, the first element is the current epoch.
pub fn get_claimable_epochs(deps: Deps) -> StdResult<ClaimableEpochsResponse> {
    let grace_period = CONFIG.load(deps.storage)?.grace_period;

    let epochs = EPOCHS
        .range(deps.storage, None, None, Order::Descending)
        .take(grace_period.u64() as usize)
        .map(|item| {
            let (_, epoch) = item?;
            Ok(epoch)
        })
        .collect::<StdResult<Vec<Epoch>>>()?;

    Ok(ClaimableEpochsResponse { epochs })
}

/// Returns the epochs that can be claimed by the given address.
pub fn query_claimable(deps: Deps, address: &Addr) -> StdResult<ClaimableEpochsResponse> {
    let mut claimable_epochs = get_claimable_epochs(deps)?.epochs;
    let last_claimed_epoch = LAST_CLAIMED_EPOCH.may_load(deps.storage, address)?;

    // filter out epochs that have already been claimed by the user
    if let Some(last_claimed_epoch) = last_claimed_epoch {
        claimable_epochs.retain(|epoch| epoch.id > last_claimed_epoch);
    } else {
        // if the user doesn't have any last_claimed_epoch two things might be happening:
        // 1- the user has never bonded before
        // 2- the user has bonded, but never claimed any rewards so far
        let bonding_contract = CONFIG.load(deps.storage)?.bonding_contract_addr;

        let bonded_response: BondedResponse =
            deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: bonding_contract.to_string(),
                msg: to_json_binary(&QueryMsg::Bonded {
                    address: address.to_string(),
                })?,
            }))?;

        if bonded_response.bonded_assets.is_empty() {
            // the user has never bonded before, therefore it shouldn't be able to claim anything
            claimable_epochs.clear();
        } else {
            // the user has bonded, but never claimed any rewards so far
            claimable_epochs.retain(|epoch| epoch.id > bonded_response.first_bonded_epoch_id);
        }
    };

    // filter out epochs that have no available fees. This would only happen in case the grace period
    // gets increased after epochs have expired, which would lead to make them available for claiming
    // again without any available rewards, as those were forwarded to newer epochs.
    claimable_epochs.retain(|epoch| !epoch.available.is_empty());

    Ok(ClaimableEpochsResponse {
        epochs: claimable_epochs,
    })
}
