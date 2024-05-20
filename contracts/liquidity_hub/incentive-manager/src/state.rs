use std::clone::Clone;
use std::string::ToString;

use cosmwasm_std::{Addr, Order, StdResult, Storage, Uint128};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, Item, Map, MultiIndex};

use white_whale_std::incentive_manager::{Config, EpochId, Incentive, Position};

use crate::ContractError;

/// Contract's config
pub const CONFIG: Item<Config> = Item::new("config");

/// A monotonically increasing counter to generate unique position identifiers.
pub const POSITION_ID_COUNTER: Item<u64> = Item::new("position_id_counter");

/// The positions that a user has. Positions can be open or closed.
/// The key is the position identifier
pub const POSITIONS: IndexedMap<&str, Position, PositionIndexes> = IndexedMap::new(
    "positions",
    PositionIndexes {
        receiver: MultiIndex::new(
            |_pk, p| p.receiver.to_string(),
            "positions",
            "positions__receiver",
        ),
    },
);

pub struct PositionIndexes<'a> {
    pub receiver: MultiIndex<'a, String, Position, String>,
}

impl<'a> IndexList<Position> for PositionIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Position>> + '_> {
        let v: Vec<&dyn Index<Position>> = vec![&self.receiver];
        Box::new(v.into_iter())
    }
}

/// The last epoch an address claimed rewards
pub const LAST_CLAIMED_EPOCH: Map<&Addr, EpochId> = Map::new("last_claimed_epoch");

/// The lp weight history for addresses, including the contract. i.e. how much lp weight an address
/// or contract has at a given epoch.
/// Key is a tuple of (address, lp_denom, epoch_id), value is the lp weight.
pub const LP_WEIGHT_HISTORY: Map<(&Addr, &str, EpochId), Uint128> = Map::new("lp_weight_history");

/// A monotonically increasing counter to generate unique incentive identifiers.
pub const INCENTIVE_COUNTER: Item<u64> = Item::new("incentive_counter");

/// Incentives map
pub const INCENTIVES: IndexedMap<&str, Incentive, IncentiveIndexes> = IndexedMap::new(
    "incentives",
    IncentiveIndexes {
        lp_denom: MultiIndex::new(
            |_pk, i| i.lp_denom.to_string(),
            "incentives",
            "incentives__lp_asset",
        ),
        incentive_asset: MultiIndex::new(
            |_pk, i| i.incentive_asset.denom.clone(),
            "incentives",
            "incentives__incentive_asset",
        ),
    },
);

pub struct IncentiveIndexes<'a> {
    pub lp_denom: MultiIndex<'a, String, Incentive, String>,
    pub incentive_asset: MultiIndex<'a, String, Incentive, String>,
}

impl<'a> IndexList<Incentive> for IncentiveIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Incentive>> + '_> {
        let v: Vec<&dyn Index<Incentive>> = vec![&self.lp_denom, &self.incentive_asset];
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

/// Gets incentives given an lp denom.
pub fn get_incentives_by_lp_denom(
    storage: &dyn Storage,
    lp_denom: &str,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<Incentive>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = cw_utils::calc_range_start_string(start_after).map(Bound::ExclusiveRaw);

    INCENTIVES
        .idx
        .lp_denom
        .prefix(lp_denom.to_owned())
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, incentive) = item?;

            Ok(incentive)
        })
        .collect()
}

/// Gets all the incentives that are offering the given incentive_asset as a reward.
pub fn get_incentives_by_incentive_asset(
    storage: &dyn Storage,
    incentive_asset: &str,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<Incentive>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = cw_utils::calc_range_start_string(start_after).map(Bound::ExclusiveRaw);

    INCENTIVES
        .idx
        .incentive_asset
        .prefix(incentive_asset.to_owned())
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

/// Gets all the positions of the given receiver.
pub fn get_positions_by_receiver(
    storage: &dyn Storage,
    receiver: String,
    open_state: Option<bool>,
) -> StdResult<Vec<Position>> {
    let limit = MAX_LIMIT as usize;

    let mut positions_by_receiver = POSITIONS
        .idx
        .receiver
        .prefix(receiver)
        .range(storage, None, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, position) = item?;
            Ok(position)
        })
        .collect::<StdResult<Vec<Position>>>()?;

    if let Some(open) = open_state {
        positions_by_receiver = positions_by_receiver
            .into_iter()
            .filter(|position| position.open == open)
            .collect::<Vec<Position>>();
    }

    Ok(positions_by_receiver)
}

/// Gets the earliest entry of an address in the address lp weight history.
/// If the address has no open positions, it returns an error.
pub fn get_earliest_address_lp_weight(
    storage: &dyn Storage,
    address: &Addr,
    lp_denom: &str,
) -> Result<(EpochId, Uint128), ContractError> {
    let earliest_weight_history_result = LP_WEIGHT_HISTORY
        .prefix((address, lp_denom))
        .range(storage, None, None, Order::Ascending)
        .next()
        .transpose();

    match earliest_weight_history_result {
        Ok(Some(item)) => Ok(item),
        Ok(None) => Err(ContractError::NoOpenPositions),
        Err(std_err) => Err(std_err.into()),
    }
}

/// Gets the latest entry of an address in the address lp weight history.
/// If the address has no open positions, returns 0 for the weight.
pub fn get_latest_address_lp_weight(
    storage: &dyn Storage,
    address: &Addr,
    lp_denom: &str,
    epoch_id: &EpochId,
) -> Result<(EpochId, Uint128), ContractError> {
    let latest_weight_history_result = LP_WEIGHT_HISTORY
        .prefix((address, lp_denom))
        .range(storage, None, None, Order::Descending)
        .next()
        .transpose();

    match latest_weight_history_result {
        Ok(Some(item)) => Ok(item),
        Ok(None) => Ok((epoch_id.to_owned(), Uint128::zero())),
        Err(std_err) => Err(std_err.into()),
    }
}
