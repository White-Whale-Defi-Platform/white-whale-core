use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terraswap::asset::{Asset, AssetInfo};
use white_whale::fee::VaultFee;

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// The owner of the vault
    pub owner: Addr,
    /// The asset info the vault manages
    pub asset_info: AssetInfo,
    /// If flash-loans are enabled
    pub flash_loan_enabled: bool,
    /// If deposits are enabled
    pub deposit_enabled: bool,
    /// If withdrawals are enabled
    pub withdraw_enabled: bool,
    /// The address of the liquidity token
    pub liquidity_token: Addr,
    /// The address of the fee collector
    pub fee_collector_addr: Addr,
    /// The fees associated with this vault
    pub fees: VaultFee,
}

pub const COLLECTED_PROTOCOL_FEES: Item<Asset> = Item::new("collected_protocol_fees");
pub const ALL_TIME_COLLECTED_PROTOCOL_FEES: Item<Asset> =
    Item::new("all_time_collected_protocol_fees");
