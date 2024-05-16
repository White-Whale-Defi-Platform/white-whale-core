use crate::ContractError;
use cosmwasm_std::{Addr, Coin, Decimal, DepsMut, StdError, StdResult, Uint128};
use cw_storage_plus::{Item, Map};
use white_whale_std::bonding_manager::{
    Bond, Config, GlobalIndex, RewardBucket, UpcomingRewardBucket,
};
use white_whale_std::pool_network::asset;

type Denom = str;

pub const BONDING_ASSETS_LIMIT: usize = 2;
pub const CONFIG: Item<Config> = Item::new("config");
pub const BOND: Map<(&Addr, &Denom), Bond> = Map::new("bond");
pub const UNBOND: Map<(&Addr, &Denom, u64), Bond> = Map::new("unbond");
pub const GLOBAL: Item<GlobalIndex> = Item::new("global");
pub const LAST_CLAIMED_EPOCH: Map<&Addr, u64> = Map::new("last_claimed_epoch");
pub const REWARD_BUCKETS: Map<u64, RewardBucket> = Map::new("reward_buckets");

/// This is the upcoming reward bucket that will hold the rewards coming to the contract after a
/// new epoch gets created. Once a new epoch is created, this bucket will be forwarded to the
/// reward buckets map, and reset for the new rewards to come.
pub const UPCOMING_REWARD_BUCKET: Item<UpcomingRewardBucket> = Item::new("upcoming_reward_bucket");

/// Updates the local weight of the given address.
pub fn update_bond_weight(
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
        bond.last_updated,
    )?;

    bond.last_updated = current_epoch_id;
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

    global_index.last_weight = get_weight(
        current_epoch_id,
        global_index.last_weight,
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

/// Fills the upcoming reward bucket with the given funds.
pub fn fill_upcoming_reward_bucket(deps: DepsMut, funds: Coin) -> StdResult<()> {
    UPCOMING_REWARD_BUCKET.update(deps.storage, |mut upcoming_bucket| -> StdResult<_> {
        upcoming_bucket.total = asset::aggregate_coins(&upcoming_bucket.total, &vec![funds])?;
        Ok(upcoming_bucket)
    })?;

    Ok(())
}
