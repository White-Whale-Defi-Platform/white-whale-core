use crate::ContractError;
use cosmwasm_std::{
    Addr, Decimal, Deps, DepsMut, Order, StdError, StdResult, Timestamp, Uint128, Uint64,
};
use cw_storage_plus::{Item, Map};
use white_whale_std::bonding_manager::{Bond, Config, Epoch, GlobalIndex};

type Denom = str;

pub const BONDING_ASSETS_LIMIT: usize = 2;
pub const CONFIG: Item<Config> = Item::new("config");
pub const BOND: Map<(&Addr, &Denom), Bond> = Map::new("bond");
pub const UNBOND: Map<(&Addr, &Denom, u64), Bond> = Map::new("unbond");
pub const GLOBAL: Item<GlobalIndex> = Item::new("global");
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

    //todo remove? done outside of this function. Or remove outside
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

    //todo remove? done outside of this function. Or remove outside
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
