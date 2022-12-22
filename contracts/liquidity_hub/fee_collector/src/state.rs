use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub pool_router: Addr,
}

pub type ConfigResponse = Config;

pub const CONFIG: Item<Config> = Item::new("config");
