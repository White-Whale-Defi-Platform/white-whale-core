use cosmwasm_schema::cw_serde;
use cosmwasm_std::CanonicalAddr;
use cw_storage_plus::{Item, Map};

use pool_network::router::SwapOperation;

#[cw_serde]
pub struct Config {
    pub terraswap_factory: CanonicalAddr,
}

#[cw_serde]
pub struct Routes {
    pub routes: Vec<SwapOperation>,
}

pub type RoutesResponse = Routes;

pub const CONFIG: Item<Config> = Item::new("config");
pub const SWAP_ROUTES: Map<(&str, &str), Vec<SwapOperation>> = Map::new("swap_routes");
