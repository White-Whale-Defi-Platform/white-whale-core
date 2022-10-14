use cosmwasm_std::{StdError, StdResult, Storage, Uint128};
use cw_storage_plus::Item;

use terraswap::asset::{Asset, PairInfoRaw};
use terraswap::pair::Config;

pub const PAIR_INFO: Item<PairInfoRaw> = Item::new("pair_info");
pub const CONFIG: Item<Config> = Item::new("config");
pub const COLLECTED_PROTOCOL_FEES: Item<Vec<Asset>> = Item::new("collected_protocol_fees");
pub const ALL_TIME_COLLECTED_PROTOCOL_FEES: Item<Vec<Asset>> =
    Item::new("all_time_collected_protocol_fees");

/// Stores the protocol fee for a given asset
pub fn store_protocol_fee(
    storage: &mut dyn Storage,
    protocol_fee: Uint128,
    asset_id: String,
    protocol_fees_storage_item: Item<Vec<Asset>>,
) -> StdResult<()> {
    let protocol_fees = protocol_fees_storage_item
        .load(storage)?
        .iter()
        .map(|protocol_fee_asset| {
            if protocol_fee_asset.clone().get_id() == asset_id {
                Asset {
                    info: protocol_fee_asset.info.clone(),
                    amount: protocol_fee_asset.amount + protocol_fee,
                }
            } else {
                protocol_fee_asset.clone()
            }
        })
        .collect();

    protocol_fees_storage_item.save(storage, &protocol_fees)
}

/// Stores the protocol fee for a given asset
pub fn get_protocol_fees_for_asset(storage: &dyn Storage, asset_id: String) -> StdResult<Asset> {
    let protocol_fees = COLLECTED_PROTOCOL_FEES
        .load(storage)?
        .iter()
        .find(|&protocol_fee_asset| protocol_fee_asset.clone().get_id() == asset_id)
        .cloned();

    if let Some(protocol_fees) = protocol_fees {
        Ok(protocol_fees)
    } else {
        Err(StdError::generic_err(format!(
            "Protocol fees for asset {} not found",
            asset_id
        )))
    }
}
