use crate::vault::{self};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use pool_network::asset::AssetInfo;
use white_whale::fee::VaultFee;

/// The instantiation message
#[cw_serde]
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
#[cw_serde]
pub enum ExecuteMsg {
    /// Creates a new vault given the asset info the vault should manage deposits and withdrawals
    /// for and the fees
    CreateVault {
        asset_info: AssetInfo,
        fees: VaultFee,
    },
    /// Migrates vaults to the given code_id. If a [vault_addr] is provided, then migrates only that
    /// vault.
    MigrateVaults {
        vault_addr: Option<String>,
        vault_code_id: u64,
    },
    /// Removes a vault given its [AssetInfo]
    RemoveVault { asset_info: AssetInfo },
    /// Updates a vault config
    UpdateVaultConfig {
        vault_addr: String,
        params: vault::UpdateConfigParams,
    },
    /// Updates the configuration of the vault factory.
    /// If a field is not specified, it will not be modified.
    UpdateConfig {
        owner: Option<String>,
        fee_collector_addr: Option<String>,
        vault_id: Option<u64>,
        token_id: Option<u64>,
    },
}

/// The query message
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Retrieves the configuration of the vault.
    #[returns(Config)]
    Config {},
    /// Retrieves the address of a given vault.
    #[returns(Option<String>)]
    Vault { asset_info: AssetInfo },
    /// Retrieves the addresses for all the vaults.
    #[returns(VaultsResponse)]
    Vaults {
        start_after: Option<Vec<u8>>,
        limit: Option<u32>,
    },
}

/// The migrate message
#[cw_serde]
pub struct MigrateMsg {}

/// The `reply` code ID for the submessage after instantiating the vault.
pub const INSTANTIATE_VAULT_REPLY_ID: u64 = 1;

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub vault_id: u64,
    pub token_id: u64,
    pub fee_collector_addr: Addr,
}

/// Response for the vaults query
#[cw_serde]
pub struct VaultsResponse {
    pub vaults: Vec<VaultInfo>,
}

/// Response for the vaults query
#[cw_serde]
pub struct VaultInfo {
    pub vault: String,
    pub asset_info: AssetInfo,
    pub asset_info_reference: Vec<u8>,
}
