use cosmwasm_std::{Addr, Deps, Order, StdResult, Uint64};
use cw_storage_plus::{Item, Map};

use white_whale::fee_distributor::{ClaimableEpochsResponse, Config, Epoch, EpochResponse};

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
    let option = EPOCHS.may_load(deps.storage, &id.to_be_bytes())?;

    let epoch = match option {
        Some(epoch) => epoch,
        None => Epoch::default(),
    };

    Ok(EpochResponse { epoch })
}

/// Returns the epoch that is falling out the grace period, which is the one expiring after creating
/// a new epoch is created.
pub fn get_expiring_epoch(deps: Deps) -> StdResult<Option<Epoch>> {
    let grace_period = CONFIG.load(deps.storage)?.grace_period;

    let option = EPOCHS
        .range(deps.storage, None, None, Order::Descending)
        .take(grace_period.u64() as usize)
        .last();

    let epoch = option
        .and_then(|result| result.ok())
        .map(|(_, epoch)| epoch);

    Ok(epoch)
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
    };

    Ok(ClaimableEpochsResponse {
        epochs: claimable_epochs,
    })
}
