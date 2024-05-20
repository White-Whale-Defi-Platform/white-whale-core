use cosmwasm_std::{Addr, Decimal, DepsMut, Order, StdError, StdResult, Storage, Uint128};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, Item, Map, MultiIndex};

use white_whale_std::bonding_manager::{
    Bond, Config, GlobalIndex, RewardBucket, UpcomingRewardBucket,
};

use crate::ContractError;

pub const BONDING_ASSETS_LIMIT: usize = 2;
pub const CONFIG: Item<Config> = Item::new("config");

/// A monotonically increasing counter to generate unique bond ids.
pub const BOND_COUNTER: Item<u64> = Item::new("bond_counter");
pub const BONDS: IndexedMap<u64, Bond, BondIndexes> = IndexedMap::new(
    "bonds",
    BondIndexes {
        receiver: MultiIndex::new(|_pk, b| b.receiver.to_string(), "bonds", "bonds__receiver"),
    },
);

pub struct BondIndexes<'a> {
    pub receiver: MultiIndex<'a, String, Bond, String>,
}

impl<'a> IndexList<Bond> for BondIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Bond>> + '_> {
        let v: Vec<&dyn Index<Bond>> = vec![&self.receiver];
        Box::new(v.into_iter())
    }
}

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
    BONDS.save(deps.storage, bond.id, &bond)?;

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

// settings for pagination
pub(crate) const MAX_LIMIT: u8 = 100;
pub const DEFAULT_LIMIT: u8 = 10;

pub fn get_bonds_by_receiver(
    storage: &dyn Storage,
    receiver: String,
    is_bonding: Option<bool>,
    asset_denom: Option<String>,
    start_after: Option<u64>,
    limit: Option<u8>,
) -> StdResult<Vec<Bond>> {
    let start = calc_range_start(start_after).map(Bound::ExclusiveRaw);
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let mut bonds_by_receiver = BONDS
        .idx
        .receiver
        .prefix(receiver)
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, bond) = item?;
            Ok(bond)
        })
        .collect::<StdResult<Vec<Bond>>>()?;

    if let Some(is_bonding) = is_bonding {
        bonds_by_receiver.retain(|bond| bond.unbonded_at.is_none() == is_bonding);
    }

    if let Some(asset_denom) = asset_denom {
        bonds_by_receiver.retain(|bond| bond.asset.denom == asset_denom);
    }

    Ok(bonds_by_receiver)
}

fn calc_range_start(start_after: Option<u64>) -> Option<Vec<u8>> {
    start_after.map(|block_height| {
        let mut v: Vec<u8> = block_height.to_be_bytes().to_vec();
        v.push(0);
        v
    })
}
