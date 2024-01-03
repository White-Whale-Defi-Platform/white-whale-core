use cosmwasm_std::{Addr, Order, StdResult, Storage, Uint128};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, Item, Map, MultiIndex};

use white_whale::incentive_manager::{Config, EpochId, Incentive, Position};
use white_whale::pool_network::asset::AssetInfo;

use crate::ContractError;

// Contract's config
pub const CONFIG: Item<Config> = Item::new("config");

/// All open positions that a user have. Open positions accumulate rewards, and a user can have
/// multiple open positions active at once.
pub const OPEN_POSITIONS: Map<&Addr, Vec<Position>> = Map::new("open_positions");

/// All closed positions that users have. Closed positions don't accumulate rewards, and the
/// underlying tokens are claimable after `unbonding_duration`.
pub const CLOSED_POSITIONS: Map<&Addr, Vec<Position>> = Map::new("closed_positions");

/// The last epoch an address claimed rewards
pub const LAST_CLAIMED_EPOCH: Map<&Addr, EpochId> = Map::new("last_claimed_epoch");

/// The total weight (sum of all individual weights) of an LP asset
pub const LP_WEIGHTS: Map<&[u8], Uint128> = Map::new("lp_weights");

/// The weights for individual accounts
pub const ADDRESS_LP_WEIGHT: Map<(&Addr, &[u8]), Uint128> = Map::new("address_lp_weight");

/// The address lp weight history, i.e. how much lp weight an address had at a given epoch
pub const ADDRESS_LP_WEIGHT_HISTORY: Map<(&Addr, EpochId), Uint128> =
    Map::new("address_lp_weight_history");

/// An monotonically increasing counter to generate unique incentive identifiers.
pub const INCENTIVE_COUNTER: Item<u64> = Item::new("incentive_counter");

/// Incentives map
pub const INCENTIVES: IndexedMap<String, Incentive, IncentiveIndexes> = IndexedMap::new(
    "incentives",
    IncentiveIndexes {
        lp_asset: MultiIndex::new(
            |_pk, i| i.lp_asset.to_string(),
            "incentives",
            "incentives__lp_asset",
        ),
        incentive_asset: MultiIndex::new(
            |_pk, i| i.incentive_asset.to_string(),
            "incentives",
            "incentives__incentive_asset",
        ),
    },
);

pub struct IncentiveIndexes<'a> {
    pub lp_asset: MultiIndex<'a, String, Incentive, String>,
    pub incentive_asset: MultiIndex<'a, String, Incentive, String>,
}

impl<'a> IndexList<Incentive> for IncentiveIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Incentive>> + '_> {
        let v: Vec<&dyn Index<Incentive>> = vec![&self.lp_asset, &self.incentive_asset];
        Box::new(v.into_iter())
    }
}

// settings for pagination
pub(crate) const MAX_LIMIT: u32 = 100;
const DEFAULT_LIMIT: u32 = 10;

/// Gets the incentives in the contract
pub fn get_incentives(
    storage: &dyn Storage,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<Incentive>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = cw_utils::calc_range_start_string(start_after).map(Bound::ExclusiveRaw);

    INCENTIVES
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, incentive) = item?;

            Ok(incentive)
        })
        .collect()
}

/// Gets incentives given an lp asset [AssetInfo]
pub fn get_incentives_by_lp_asset(
    storage: &dyn Storage,
    lp_asset: &AssetInfo,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<Incentive>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = cw_utils::calc_range_start_string(start_after).map(Bound::ExclusiveRaw);

    INCENTIVES
        .idx
        .lp_asset
        .prefix(lp_asset.to_string())
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, incentive) = item?;

            Ok(incentive)
        })
        .collect()
}

/// Gets incentives given an incentive asset as [AssetInfo]
pub fn get_incentive_by_asset(
    storage: &dyn Storage,
    incentive_asset: &AssetInfo,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<Incentive>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = cw_utils::calc_range_start_string(start_after).map(Bound::ExclusiveRaw);

    INCENTIVES
        .idx
        .incentive_asset
        .prefix(incentive_asset.to_string())
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, incentive) = item?;

            Ok(incentive)
        })
        .collect()
}

/// Gets the incentive given its identifier
pub fn get_incentive_by_identifier(
    storage: &dyn Storage,
    incentive_identifier: &String,
) -> Result<Incentive, ContractError> {
    INCENTIVES
        .may_load(storage, incentive_identifier.clone())?
        .ok_or(ContractError::NonExistentIncentive {})
}
