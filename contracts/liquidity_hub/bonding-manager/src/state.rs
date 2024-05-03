use crate::queries::query_bonded;
use crate::ContractError;
use cosmwasm_std::{
    Addr, Decimal, Deps, DepsMut, Order, StdError, StdResult, Timestamp, Uint128, Uint64,
};
use cw_storage_plus::{Item, Map};
use white_whale_std::bonding_manager::{
    Bond, BondedResponse, ClaimableEpochsResponse, Config, Epoch, EpochResponse, GlobalIndex,
};

type Denom = str;

pub const BONDING_ASSETS_LIMIT: usize = 2;
pub const CONFIG: Item<Config> = Item::new("config");
pub const BOND: Map<(&Addr, &Denom), Bond> = Map::new("bond");
pub const UNBOND: Map<(&Addr, &Denom, u64), Bond> = Map::new("unbond");
pub const GLOBAL: Item<GlobalIndex> = Item::new("global");
pub type EpochID = [u8];

pub const REWARDS_BUCKET: Map<&EpochID, &Epoch> = Map::new("rewards_bucket");

pub const LAST_CLAIMED_EPOCH: Map<&Addr, Uint64> = Map::new("last_claimed_epoch");
pub const EPOCHS: Map<&[u8], Epoch> = Map::new("epochs");

/// Updates the local weight of the given address.
pub fn update_local_weight(
    deps: &mut DepsMut,
    address: Addr,
    timestamp: Timestamp,
    mut bond: Bond,
) -> Result<Bond, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    bond.weight = get_weight(
        timestamp,
        bond.weight,
        bond.asset.amount,
        config.growth_rate,
        bond.timestamp,
    )?;

    bond.timestamp = timestamp;

    let denom: &String = &bond.asset.denom;

    BOND.save(deps.storage, (&address, denom), &bond)?;

    Ok(bond)
}

/// Updates the global weight of the contract.
pub fn update_global_weight(
    deps: &mut DepsMut,
    timestamp: Timestamp,
    mut global_index: GlobalIndex,
) -> Result<GlobalIndex, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    global_index.weight = get_weight(
        timestamp,
        global_index.weight,
        global_index.bonded_amount,
        config.growth_rate,
        global_index.timestamp,
    )?;

    global_index.timestamp = timestamp;

    GLOBAL.save(deps.storage, &global_index)?;

    Ok(global_index)
}

/// Calculates the bonding weight of the given amount for the provided timestamps.
pub fn get_weight(
    current_timestamp: Timestamp,
    weight: Uint128,
    amount: Uint128,
    growth_rate: Decimal,
    timestamp: Timestamp,
) -> StdResult<Uint128> {
    let time_factor = if timestamp == Timestamp::default() {
        Uint128::zero()
    } else {
        Uint128::from(
            current_timestamp
                .seconds()
                .checked_sub(timestamp.seconds())
                .ok_or_else(|| StdError::generic_err("Error calculating time_factor"))?,
        )
    };

    Ok(weight.checked_add(amount.checked_mul(time_factor)? * growth_rate)?)
}

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

        let bonded_response: BondedResponse = query_bonded(deps, address.to_string())?;

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
