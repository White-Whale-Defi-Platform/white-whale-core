use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Deps, Order, StdResult, Timestamp, Uint64};
use cw_storage_plus::{Item, Map};

use crate::msg::EpochConfig;
use terraswap::asset::Asset;

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub staking_contract_addr: Addr,
    pub fee_collector_addr: Addr,
    pub grace_period: Uint64,
    pub epoch_config: EpochConfig,
}

#[cw_serde]
pub struct Epoch {
    // Epoch identifier
    pub id: u128,
    pub start_time: Timestamp,
    // Initial fees to be distributed in this epoch.
    pub total: Vec<Asset>,
    // Fees left to be claimed on this epoch. These available fees are forwarded when the epoch expires.
    pub available: Vec<Asset>,
    // Fees that were claimed on this epoch. For keeping record on the total fees claimed.
    pub claimed: Vec<Asset>,
}

impl Default for Epoch {
    fn default() -> Self {
        Self {
            id: 0,
            start_time: Timestamp::default(),
            total: vec![],
            available: vec![],
            claimed: vec![],
        }
    }
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const LAST_CLAIMED_EPOCH: Map<&Addr, u128> = Map::new("last_claimed_epoch");
pub const EPOCHS: Map<&[u8], Epoch> = Map::new("epochs");

/// Returns the current epoch, which is the last on the EPOCHS map.
pub fn get_current_epoch(deps: Deps) -> StdResult<Epoch> {
    let option = EPOCHS
        .range(deps.storage, None, None, Order::Descending)
        .next();

    let epoch = match option {
        Some(Ok((_, epoch))) => epoch,
        _ => Epoch::default(),
    };

    Ok(epoch)
}

/// Returns the [Epoch] with the given id.
pub fn get_epoch(deps: Deps, id: u128) -> StdResult<Epoch> {
    let option = EPOCHS.may_load(deps.storage, &id.to_be_bytes())?;

    let epoch = match option {
        Some(epoch) => epoch,
        None => Epoch::default(),
    };

    Ok(epoch)
}

/// Returns the epoch that is falling out the grace period, which is the one expiring after creating
/// a new epoch is created.
pub fn get_expiring_epoch(deps: Deps) -> StdResult<Option<Epoch>> {
    let grace_period = CONFIG.load(deps.storage)?.grace_period;

    let option = EPOCHS
        .range(deps.storage, None, None, Order::Descending)
        .take(grace_period.checked_add(Uint64::one())?.u64() as usize)
        .nth(grace_period.u64() as usize);

    let epoch = option
        .and_then(|result| result.ok())
        .map(|(_, epoch)| epoch);

    Ok(epoch)
}

/// Returns the epochs that are within the grace period, i.e. the ones which fees can still be claimed.
/// The result is ordered by epoch id, descending. Thus, the first element is the current epoch.
pub fn get_claimable_epochs(deps: Deps) -> StdResult<Vec<Epoch>> {
    let grace_period = CONFIG.load(deps.storage)?.grace_period;

    EPOCHS
        .range(deps.storage, None, None, Order::Descending)
        .take(grace_period.u64() as usize)
        .map(|item| {
            let (_, epoch) = item?;
            Ok(epoch)
        })
        .collect::<StdResult<Vec<Epoch>>>()
}
