use std::string::ToString;

use cosmwasm_std::{Addr, Order, StdResult, Storage, Uint128};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, Item, Map, MultiIndex};

use white_whale_std::incentive_manager::{Config, EpochId, Incentive, Position};
use white_whale_std::pool_network::asset::AssetInfo;

use crate::ContractError;

/// Contract's config
pub const CONFIG: Item<Config> = Item::new("config");

/// An monotonically increasing counter to generate unique position identifiers.
pub const POSITION_ID_COUNTER: Item<u64> = Item::new("position_id_counter");

/// The positions that a user has. Positions can be open or closed.
/// The key is the position identifier
pub const POSITIONS: IndexedMap<&String, Position, PositionIndexes> = IndexedMap::new(
    "positions",
    PositionIndexes {
        lp_asset: MultiIndex::new(
            |_pk, p| p.lp_asset.to_string(),
            "positions",
            "positions__lp_asset",
        ),
        receiver: MultiIndex::new(
            |_pk, p| p.receiver.to_string(),
            "positions",
            "positions__receiver",
        ),
        open: MultiIndex::new(|_pk, p| p.open.to_string(), "positions", "positions__open"),
    },
);

pub struct PositionIndexes<'a> {
    pub lp_asset: MultiIndex<'a, String, Position, String>,
    pub receiver: MultiIndex<'a, String, Position, String>,
    pub open: MultiIndex<'a, String, Position, String>,
}

impl<'a> IndexList<Position> for PositionIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Position>> + '_> {
        let v: Vec<&dyn Index<Position>> = vec![&self.lp_asset, &self.receiver, &self.open];
        Box::new(v.into_iter())
    }
}

/// The last epoch an address claimed rewards
pub const LAST_CLAIMED_EPOCH: Map<&Addr, EpochId> = Map::new("last_claimed_epoch");

/// The history of total weight (sum of all individual weights) of an LP asset at a given epoch
pub const LP_WEIGHTS_HISTORY: Map<(&[u8], EpochId), Uint128> = Map::new("lp_weights_history");

/// The address lp weight history, i.e. how much lp weight an address had at a given epoch
pub const ADDRESS_LP_WEIGHT_HISTORY: Map<(&Addr, EpochId), Uint128> =
    Map::new("address_lp_weight_history");

/// An monotonically increasing counter to generate unique incentive identifiers.
pub const INCENTIVE_COUNTER: Item<u64> = Item::new("incentive_counter");

/// Incentives map
pub const INCENTIVES: IndexedMap<&String, Incentive, IncentiveIndexes> = IndexedMap::new(
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
        .may_load(storage, incentive_identifier)?
        .ok_or(ContractError::NonExistentIncentive {})
}

/// Gets a position given its identifier. If the position is not found with the given identifier, it returns None.
pub fn get_position(
    storage: &dyn Storage,
    identifier: Option<String>,
) -> StdResult<Option<Position>> {
    if let Some(identifier) = identifier {
        // there is a position
        POSITIONS.may_load(storage, &identifier)
    } else {
        // there is no position
        Ok(None)
    }
}

//todo think of the limit when claiming rewards
/// Gets the positions of the given receiver.
pub fn get_open_positions_by_receiver(
    storage: &dyn Storage,
    receiver: String,
) -> StdResult<Vec<Position>> {
    let limit = MAX_LIMIT as usize;

    let open_positions = POSITIONS
        .idx
        .receiver
        .prefix(receiver)
        .range(storage, None, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, position) = item?;
            Ok(position)
        })
        .collect::<StdResult<Vec<Position>>>()?
        .into_iter()
        .filter(|position| position.open)
        .collect::<Vec<Position>>();

    Ok(open_positions)
}
