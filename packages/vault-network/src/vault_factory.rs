use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terraswap::asset::AssetInfo;

/// The instantiation message
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// The owner of the factory
    pub owner: String,
    /// The code ID for the vault contract
    pub vault_id: u64,
}

/// The execution message
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ExecuteMsg {
    /// Creates a new vault given the asset info the vault should manage deposits and withdrawals for
    CreateVault { asset_info: AssetInfo },
    /// Updates the configuration of the vault factory.
    /// If the owner is not passed, it will not be modified.
    UpdateConfig { owner: Option<String> },
}

/// The query message
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum QueryMsg {
    /// Retrieves the configuration of the vault. Returns a [`Config`] struct.
    Config {},
}

/// The migrate message
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

/// The `reply` code ID for the submessage after instantiating the vault.
pub const INSTANTIATE_VAULT_REPLY_ID: u64 = 1;
