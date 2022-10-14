use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Deps, Order, StdResult};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    pub owner: Addr,
}

pub type ConfigResponse = Config;

pub const CONFIG: Item<Config> = Item::new("config");
pub const FACTORIES: Map<&[u8], Addr> = Map::new("factories");

// Settings for pagination
// Unless we modify our architecture, we will only have 2 factories per liquidity hub, i.e.
// i.e. a pool factory and a vault factory
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 2;

pub fn read_factories(deps: Deps, limit: Option<u32>) -> StdResult<Vec<Addr>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    FACTORIES
        .range(deps.storage, None, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, factory_addr) = item?;
            Ok(factory_addr)
        })
        .collect()
}
