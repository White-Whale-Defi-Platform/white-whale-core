use crate::fee::Fee;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, CosmosMsg, Decimal, StdError, StdResult, Uint128};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use std::fmt::{Display, Formatter};

/// The instantiation message
#[cw_serde]
pub struct InstantiateMsg {
    /// The owner of the contract
    pub owner: String,
    /// The bonding manager address, where protocol fees are distributed
    pub bonding_manager_addr: String,
    /// The fee to create a vault
    pub vault_creation_fee: Coin,
}

/// Configuration for the contract (manager)
#[cw_serde]
pub struct Config {
    /// The bonding manager contract address
    pub bonding_manager_addr: Addr,
    /// The fee to create a new vault
    pub vault_creation_fee: Coin,
    /// If flash-loans are enabled
    pub flash_loan_enabled: bool,
    /// If deposits are enabled
    pub deposit_enabled: bool,
    /// If withdrawals are enabled
    pub withdraw_enabled: bool,
}

/// Vault representation
#[cw_serde]
pub struct Vault {
    /// The asset the vault manages
    pub asset: Coin,
    /// The LP asset
    pub lp_denom: String,
    /// The fees associated with the vault
    pub fees: VaultFee,
    /// Identifier associated with the vault
    pub identifier: String,
}

#[cw_serde]
pub struct VaultFee {
    pub protocol_fee: Fee,
    pub flash_loan_fee: Fee,
}

impl VaultFee {
    /// Checks that the given [VaultFee] is valid, i.e. the fees provided are valid, and they don't
    /// exceed 100% together
    pub fn is_valid(&self) -> StdResult<()> {
        self.protocol_fee.is_valid()?;
        self.flash_loan_fee.is_valid()?;

        if self
            .protocol_fee
            .share
            .checked_add(self.flash_loan_fee.share)?
            >= Decimal::percent(100)
        {
            return Err(StdError::generic_err("Invalid fees"));
        }
        Ok(())
    }
}

impl Display for VaultFee {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "protocol_fee: {}, flash_loan_fee: {}",
            self.protocol_fee, self.flash_loan_fee
        )
    }
}

/// The execution messages
#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    /// Creates a new vault given the asset info the vault should manage deposits and withdrawals
    /// for and the fees
    CreateVault {
        asset_denom: String,
        fees: VaultFee,
        vault_identifier: Option<String>,
    },
    /// Updates the configuration of the vault manager.
    /// If a field is not specified, it will not be modified.
    UpdateConfig {
        bonding_manager_addr: Option<String>,
        vault_creation_fee: Option<Coin>,
        flash_loan_enabled: Option<bool>,
        deposit_enabled: Option<bool>,
        withdraw_enabled: Option<bool>,
    },
    /// Deposits a given asset into the vault manager.
    Deposit { vault_identifier: String },
    /// Withdraws from the vault manager. Used when the LP token is a token manager token.
    Withdraw,
    /// Retrieves the desired `asset` and runs the `payload`, paying the required amount back to the vault
    /// after running the messages in the payload, and returning the profit to the sender.
    FlashLoan {
        asset: Coin,
        vault_identifier: String,
        payload: Vec<CosmosMsg>,
    },
    /// Callback message for post-processing flash-loans.
    Callback(CallbackMsg),
}

/// The migrate message
#[cw_serde]
pub struct MigrateMsg {}

/// The query messages
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Retrieves the configuration of the manager.
    #[returns(Config)]
    Config {},
    /// Retrieves a vault given the asset_info.
    #[returns(VaultsResponse)]
    Vault { filter_by: FilterVaultBy },
    /// Retrieves the addresses for all the vaults.
    #[returns(VaultsResponse)]
    Vaults {
        start_after: Option<Vec<u8>>,
        limit: Option<u32>,
    },
    /// Retrieves the share of the assets stored in the vault that a given `lp_share` is entitled to.
    #[returns(ShareResponse)]
    Share { lp_share: Coin },
    /// Retrieves the [`Uint128`] amount that must be sent back to the contract to pay off a loan taken out.
    #[returns(PaybackAssetResponse)]
    PaybackAmount {
        asset: Coin,
        vault_identifier: String,
    },
}

/// Response for the vaults query
#[cw_serde]
pub struct VaultsResponse {
    pub vaults: Vec<Vault>,
}

/// Response for the vaults query
#[cw_serde]
pub enum FilterVaultBy {
    Asset(AssetQueryParams),
    Identifier(String),
    LpAsset(String),
}

#[cw_serde]
pub struct AssetQueryParams {
    pub asset_denom: String,
    pub start_after: Option<Vec<u8>>,
    pub limit: Option<u32>,
}

/// The callback messages available. Only callable by the vault contract itself.
#[cw_serde]
pub enum CallbackMsg {
    AfterFlashloan {
        old_asset_balance: Uint128,
        loan_asset: Coin,
        vault_identifier: String,
        sender: Addr,
    },
}

/// Response for the PaybackAmount query. Contains the amount that must be paid back to the contract
/// if taken a flashloan.
#[cw_serde]
pub struct PaybackAssetResponse {
    /// The asset info of the asset that must be paid back
    pub asset_denom: String,
    /// The total amount that must be returned. Equivalent to `amount` + `protocol_fee` + `flash_loan_fee`.
    pub payback_amount: Uint128,
    /// The amount of fee paid to the protocol
    pub protocol_fee: Uint128,
    /// The amount of fee paid to vault holders
    pub flash_loan_fee: Uint128,
}

/// Response for the Share query. Contains the amount of assets that the given `lp_share` is entitled to.
#[cw_serde]
pub struct ShareResponse {
    /// The amount of assets that the given `lp_share` is entitled to.
    pub share: Coin,
}
