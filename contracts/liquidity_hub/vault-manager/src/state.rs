use cosmwasm_std::{Order, StdResult, Storage};
use cw_storage_plus::{Bound, Item, Map};
use white_whale::pool_network::asset::Asset;
use white_whale::vault_manager::{ManagerConfig, Vault};

pub const MANAGER_CONFIG: Item<ManagerConfig> = Item::new("manager_config");
pub const VAULTS: Map<&[u8], Vault> = Map::new("vaults");
// Fees that have been accrued by the vaults
pub const COLLECTED_PROTOCOL_FEES: Item<Asset> = Item::new("collected_protocol_fees");
// Fees that have been burned by the vault since the vault's inception
pub const ALL_TIME_BURNED_FEES: Item<Asset> = Item::new("all_time_burned_fees");
// A counter for how many active loans are being performed
pub const LOAN_COUNTER: Item<u32> = Item::new("loan_counter");

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

pub fn read_vaults(
    storage: &dyn Storage,
    start_after: Option<Vec<u8>>,
    limit: Option<u32>,
) -> StdResult<Vec<Vault>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after).map(Bound::ExclusiveRaw);

    VAULTS
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, vault) = item?;

            Ok(vault)
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
