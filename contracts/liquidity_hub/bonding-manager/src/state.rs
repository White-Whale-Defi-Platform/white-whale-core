use crate::ContractError;
use cosmwasm_std::{Addr, Decimal, Deps, DepsMut, Order, StdError, StdResult, Uint128};
use cw_storage_plus::{Item, Map};
use white_whale_std::bonding_manager::{Bond, Config, GlobalIndex, RewardBucket};

type Denom = str;

pub const BONDING_ASSETS_LIMIT: usize = 2;
pub const CONFIG: Item<Config> = Item::new("config");
pub const BOND: Map<(&Addr, &Denom), Bond> = Map::new("bond");
pub const UNBOND: Map<(&Addr, &Denom, u64), Bond> = Map::new("unbond");
pub const GLOBAL: Item<GlobalIndex> = Item::new("global");
pub const LAST_CLAIMED_EPOCH: Map<&Addr, u64> = Map::new("last_claimed_epoch");
pub const REWARD_BUCKETS: Map<u64, RewardBucket> = Map::new("reward_buckets");

/// Updates the local weight of the given address.
pub fn update_local_weight(
    deps: &mut DepsMut,
    address: Addr,
    current_epoch_id: u64,
    mut bond: Bond,
) -> Result<Bond, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    bond.weight = get_weight(
        current_epoch_id,
        bond.weight,
        bond.asset.amount,
        config.growth_rate,
        bond.updated_last,
    )?;

    bond.updated_last = current_epoch_id;
    BOND.save(deps.storage, (&address, &bond.asset.denom), &bond)?;

    Ok(bond)
}

/// Updates the global weight of the contract.
pub fn update_global_weight(
    deps: &mut DepsMut,
    current_epoch_id: u64,
    mut global_index: GlobalIndex,
) -> Result<GlobalIndex, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    global_index.weight = get_weight(
        current_epoch_id,
        global_index.weight,
        global_index.bonded_amount,
        config.growth_rate,
        global_index.last_updated,
    )?;

    global_index.last_updated = current_epoch_id;
    GLOBAL.save(deps.storage, &global_index)?;

    Ok(global_index)
}

/// Calculates the bonding weight of the given amount for the provided epochs.
pub fn get_weight(
    current_epoch_id: u64,
    weight: Uint128,
    amount: Uint128,
    growth_rate: Decimal,
    epoch_id: u64,
) -> StdResult<Uint128> {
    let time_factor = if current_epoch_id == epoch_id {
        Uint128::zero()
    } else {
        Uint128::from(
            current_epoch_id
                .checked_sub(epoch_id)
                .ok_or_else(|| StdError::generic_err("Error calculating time_factor"))?,
        )
    };

    Ok(weight.checked_add(amount.checked_mul(time_factor)? * growth_rate)?)
}

/// Returns the epoch that is falling out the grace period, which is the one expiring after creating
/// a new epoch is created.
pub fn get_expiring_epoch(deps: Deps) -> StdResult<Option<RewardBucket>> {
    let grace_period = CONFIG.load(deps.storage)?.grace_period;

    // last epochs within the grace period
    let epochs = REWARD_BUCKETS
        .range(deps.storage, None, None, Order::Descending)
        .take(grace_period as usize)
        .map(|item| {
            let (_, epoch) = item?;
            Ok(epoch)
        })
        .collect::<StdResult<Vec<RewardBucket>>>()?;

    // if the epochs vector's length is the same as the grace period it means there is one epoch that
    // is expiring once the new one is created i.e. the last epoch in the vector
    if epochs.len() == grace_period as usize {
        Ok(Some(epochs.last().cloned().unwrap_or_default()))
    } else {
        // nothing is expiring yet
        Ok(None)
    }
}
