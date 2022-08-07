use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub vault_id: u64,
    pub token_id: u64,
    pub fee_collector_addr: Addr,
}

pub const VAULTS: Map<&[u8], Addr> = Map::new("vaults");

/// Used to temporarily store the asset being instantiated between `create_vault` and `reply` callback
pub const TMP_VAULT_ASSET: Item<Vec<u8>> = Item::new("tmp_vault_asset");
