use cosmwasm_std::{Addr, Api, Order, StdResult, Storage};
use cw_storage_plus::{Bound, Item, Map};
use pool_network::asset::AssetInfo;

use vault_network::vault_factory::{Config, VaultInfo};

pub const CONFIG: Item<Config> = Item::new("config");

pub const VAULTS: Map<&[u8], (Addr, AssetInfo)> = Map::new("vaults");

/// Used to temporarily store the asset being instantiated between `create_vault` and `reply` callback
pub const TMP_VAULT_ASSET: Item<(Vec<u8>, AssetInfo)> = Item::new("tmp_vault_asset");

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

pub fn read_vaults(
    storage: &dyn Storage,
    _api: &dyn Api,
    start_after: Option<Vec<u8>>,
    limit: Option<u32>,
) -> StdResult<Vec<VaultInfo>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after).map(Bound::ExclusiveRaw);

    VAULTS
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (key, vault_data) = item?;
            let (vault_addr, asset_info) = vault_data;

            Ok(VaultInfo {
                vault: vault_addr.to_string(),
                asset_info,
                asset_info_reference: key,
            })
        })
        .collect()
}

// this will set the first key after the provided key, by appending a 1 byte
fn calc_range_start(start_after: Option<Vec<u8>>) -> Option<Vec<u8>> {
    start_after.map(|asset_info| {
        let mut v = asset_info;
        v.push(1);
        v
    })
}
