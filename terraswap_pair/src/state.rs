use cosmwasm_std::{Addr, StdError, StdResult, Storage, Uint128};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use terraswap::asset::{Asset, PairInfoRaw};
use terraswap::pair::{FeatureToggle, PoolFee};
use terraswap_helpers::asset_helper::get_asset_id;

pub const PAIR_INFO: Item<PairInfoRaw> = Item::new("pair_info");
pub const CONFIG: Item<Config> = Item::new("config");
pub const COLLECTED_PROTOCOL_FEES: Item<Vec<Asset>> = Item::new("collected_protocol_fees");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub pool_fees: PoolFee,
    pub feature_toggle: FeatureToggle,
}

pub type ConfigResponse = Config;

/// Stores the protocol fee for a given asset
pub fn store_protocol_fee(
    storage: &mut dyn Storage,
    protocol_fee: Uint128,
    asset_id: String,
) -> StdResult<()> {
    let protocol_fees = COLLECTED_PROTOCOL_FEES
        .load(storage)?
        .iter()
        .map(|protocol_fee_asset| {
            let protocol_fee_asset_id = get_asset_id(protocol_fee_asset.clone().info);
            if protocol_fee_asset_id == asset_id {
                Asset {
                    info: protocol_fee_asset.info.clone(),
                    amount: protocol_fee_asset.amount + protocol_fee,
                }
            } else {
                protocol_fee_asset.clone()
            }
        })
        .collect();

    COLLECTED_PROTOCOL_FEES.save(storage, &protocol_fees)
}

/// Stores the protocol fee for a given asset
pub fn get_protocol_fees_for_asset(storage: &dyn Storage, asset_id: String) -> StdResult<Asset> {
    let protocol_fees = COLLECTED_PROTOCOL_FEES
        .load(storage)?
        .iter()
        .find(|&protocol_fee_asset| {
            let protocol_fee_asset_id = get_asset_id(protocol_fee_asset.clone().info);
            protocol_fee_asset_id == asset_id
        })
        .cloned();

    return if let Some(protocol_fees) = protocol_fees {
        Ok(protocol_fees)
    } else {
        Err(StdError::generic_err(format!(
            "Protocol fees for asset {} not found",
            asset_id
        )))
    };
}
