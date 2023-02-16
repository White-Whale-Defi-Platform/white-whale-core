use cosmwasm_std::{StdError, StdResult, Storage, Uint128};
use cw_storage_plus::Item;

use pool_network::asset::{Asset, AssetInfo};
use vault_network::vault::Config;

pub const CONFIG: Item<Config> = Item::new("config");

// Fees that have been accrued by the vault, still unclaimed by the fee collector
pub const COLLECTED_PROTOCOL_FEES: Item<Asset> = Item::new("collected_protocol_fees");
// Fees that have been accrued by the vault since the vault's inception
pub const ALL_TIME_COLLECTED_PROTOCOL_FEES: Item<Asset> =
    Item::new("all_time_collected_protocol_fees");
// Fees that have been burned by the vault since the vault's inception
pub const ALL_TIME_BURNED_FEES: Item<Asset> = Item::new("all_time_burned_fees");

// A counter for how many active loans are being performed
pub const LOAN_COUNTER: Item<u32> = Item::new("loan_counter");

/// Stores a fee in the given fees_storage_item
pub fn store_fee(
    storage: &mut dyn Storage,
    fees_storage_item: Item<Asset>,
    fee: Uint128,
) -> StdResult<Asset> {
    fees_storage_item.update::<_, StdError>(storage, |mut fees| {
        fees.amount = fees.amount.checked_add(fee)?;
        Ok(fees)
    })
}

/// Initializes a fees_storage_item
pub fn initialize_fee(
    storage: &mut dyn Storage,
    fees_storage_item: Item<Asset>,
    asset_info: AssetInfo,
) -> StdResult<()> {
    fees_storage_item.save(
        storage,
        &Asset {
            amount: Uint128::zero(),
            info: asset_info,
        },
    )
}
