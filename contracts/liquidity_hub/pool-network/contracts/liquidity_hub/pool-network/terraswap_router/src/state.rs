use cosmwasm_schema::cw_serde;
use cosmwasm_std::CanonicalAddr;
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    pub terraswap_factory: CanonicalAddr,
}

pub const CONFIG: Item<Config> = Item::new("config");
