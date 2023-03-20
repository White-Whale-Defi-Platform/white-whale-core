use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use white_whale::pool_network::incentive_factory::Config;

pub const CONFIG: Item<Config> = Item::new("config");

/// Maps the address of the LP token to the incentive contract address
pub const INCENTIVE_MAPPINGS: Map<Addr, Addr> = Map::new("incentive_mappings");
