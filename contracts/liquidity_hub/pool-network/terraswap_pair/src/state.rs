use cosmwasm_std::{StdError, StdResult, Storage, Uint128};
use cw_storage_plus::Item;

use terraswap::asset::{Asset, PairInfoRaw};
use terraswap::pair::Config;

pub const PAIR_INFO: Item<PairInfoRaw> = Item::new("pair_info");
pub const CONFIG: Item<Config> = Item::new("config");

pub const COLLECTED_PROTOCOL_FEES: Item<Vec<Asset>> = Item::new("collected_protocol_fees");
pub const ALL_TIME_COLLECTED_PROTOCOL_FEES: Item<Vec<Asset>> =
    Item::new("all_time_collected_protocol_fees");
pub const ALL_TIME_BURNED_FEES: Item<Vec<Asset>> = Item::new("all_time_burned_fees");

/// Stores the fee for an asset in the given fees_storage_item
pub fn store_fee(
    storage: &mut dyn Storage,
    fee_amount: Uint128,
    asset_id: String,
    fees_storage_item: Item<Vec<Asset>>,
) -> StdResult<()> {
    let protocol_fees = fees_storage_item
        .load(storage)?
        .iter()
        .map(|protocol_fee_asset| {
            if protocol_fee_asset.clone().get_id() == asset_id {
                Asset {
                    info: protocol_fee_asset.info.clone(),
                    amount: protocol_fee_asset.amount + fee_amount,
                }
            } else {
                protocol_fee_asset.clone()
            }
        })
        .collect();

    fees_storage_item.save(storage, &protocol_fees)
}

/// Gets the fees for an asset from the given fees_storage_item
pub fn get_fees_for_asset(
    storage: &dyn Storage,
    asset_id: String,
    fees_storage_item: Item<Vec<Asset>>,
) -> StdResult<Asset> {
    let fees = fees_storage_item
        .load(storage)?
        .iter()
        .find(|&fee_asset| fee_asset.clone().get_id() == asset_id)
        .cloned();

    if let Some(fees) = fees {
        Ok(fees)
    } else {
        Err(StdError::generic_err(format!(
            "Fees for asset {} not found",
            asset_id
        )))
    }
}
