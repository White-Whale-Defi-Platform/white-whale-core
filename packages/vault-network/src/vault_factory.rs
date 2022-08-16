use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terraswap::asset::AssetInfo;
use white_whale::fee::VaultFee;

/// The instantiation message
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// The owner of the factory
    pub owner: String,
    /// The code ID for the vault contract
    pub vault_id: u64,
    /// The code ID for the liquidity token contract
    pub token_id: u64,
    /// The address where fees get collected
    pub fee_collector_addr: String,
}

/// The execution message
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ExecuteMsg {
    /// Creates a new vault given the asset info the vault should manage deposits and withdrawals
    /// for and the fees
    CreateVault {
        asset_info: AssetInfo,
        fees: VaultFee,
    },
    /// Updates the configuration of the vault factory.
    /// If a field is not specified, it will not be modified.
    UpdateConfig {
        owner: Option<String>,
        fee_collector_addr: Option<String>,
    },
}

/// The query message
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum QueryMsg {
    /// Retrieves the configuration of the vault. Returns a [`Config`] struct.
    Config {},
    /// Retrieves the address of a given vault. Returns an [`Option<String>`].
    Vault { asset_info: AssetInfo },
}

/// The migrate message
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

/// The `reply` code ID for the submessage after instantiating the vault.
pub const INSTANTIATE_VAULT_REPLY_ID: u64 = 1;
