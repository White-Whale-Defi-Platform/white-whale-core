use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use white_whale::pool_network::incentive::{ClosedPosition, Config, Flow, OpenPosition};

/// The configuration for the incentive contract.
pub const CONFIG: Item<Config> = Item::new("config");

/// An monotonically increasing counter to generate unique flow identifiers.
pub const FLOW_COUNTER: Item<u64> = Item::new("flow_counter");

/// The current flows that exist.
pub const FLOWS: Item<Vec<Flow>> = Item::new("flows");

/// All open positions that users have.
pub const OPEN_POSITIONS: Map<Addr, Vec<OpenPosition>> = Map::new("open_positions");
/// All closed positions that users have.
pub const CLOSED_POSITIONS: Map<Addr, Vec<ClosedPosition>> = Map::new("closed_positions");

/// The global weight (sum of all individual weights)
pub const GLOBAL_WEIGHT: Item<Uint128> = Item::new("global_weight");
/// The weights for individual accounts
pub const ADDRESS_WEIGHT: Map<Addr, Uint128> = Map::new("address_weight");

/// Tracks the last claim time for each address
pub const LAST_CLAIMED_INDEX: Map<Addr, u64> = Map::new("last_claimed_index");
