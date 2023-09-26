use crate::fee::Fee;
use crate::pool_network::asset::{Asset, AssetInfo};
use crate::vault_network::vault;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, CosmosMsg, Decimal, StdError, StdResult, Uint128};
use cw_storage_plus::UniqueIndex;
use std::fmt::{Display, Formatter};

/// The instantiation message
#[cw_serde]
pub struct InstantiateMsg {
    /// The owner of the manager
    pub owner: String,
    /// The type of LP token to use, whether a cw20 token or a token factory token
    pub lp_token_type: LpTokenType,
    /// The address where fees get collected
    pub fee_collector_addr: String,
    pub vault_creation_fee: Asset,
}

#[cw_serde]
pub struct ManagerConfig {
    /// The type of LP token to use, whether a cw20 token or a token factory token
    pub lp_token_type: LpTokenType,
    /// The fee collector contract address
    pub fee_collector_addr: Addr,
    /// The fee to create a new vault
    pub vault_creation_fee: Asset,
    /// If flash-loans are enabled
    pub flash_loan_enabled: bool,
    /// If deposits are enabled
    pub deposit_enabled: bool,
    /// If withdrawals are enabled
    pub withdraw_enabled: bool,
}

#[cw_serde]
pub struct Vault {
    /// The asset info the vault manages
    pub asset_info: AssetInfo,
    /// The LP asset
    pub lp_asset: AssetInfo,
    /// The fees associated with the vault
    pub fees: VaultFee,
}

#[cw_serde]
pub enum LpTokenType {
    Cw20(u64),
    TokenFactory,
}

impl LpTokenType {
    pub fn get_cw20_code_id(&self) -> StdResult<u64> {
        match self {
            LpTokenType::TokenFactory => Err(StdError::generic_err("Not a cw20 token")),
            LpTokenType::Cw20(code_id) => Ok(*code_id),
        }
    }
}
impl Display for LpTokenType {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            LpTokenType::Cw20(value) => write!(f, "cw20({})", value),
            LpTokenType::TokenFactory => write!(f, "token_factory"),
        }
    }
}

/// The execution message
#[cw_serde]
pub enum ExecuteMsg {
    // MANAGER MESSAGES
    /// Creates a new vault given the asset info the vault should manage deposits and withdrawals
    /// for and the fees
    CreateVault {
        asset_info: AssetInfo,
        fees: VaultFee,
    },
    /// Removes a vault given its [AssetInfo]
    RemoveVault {
        asset_info: AssetInfo,
    },
    /// Updates a vault config
    UpdateVaultFees {
        vault_asset_info: AssetInfo,
        vault_fee: VaultFee,
    },
    /// Updates the configuration of the vault manager.
    /// If a field is not specified, it will not be modified.
    UpdateManagerConfig {
        fee_collector_addr: Option<String>,
        vault_creation_fee: Option<Asset>,
        cw20_lp_code_id: Option<u64>,
        flash_loan_enabled: Option<bool>,
        deposit_enabled: Option<bool>,
        withdraw_enabled: Option<bool>,
    },

    // VAULT MESSAGES
    /// Deposits a given asset into the vault manager.
    Deposit {
        asset: Asset,
    },
    /// Withdraws from the vault manager. Used when the LP token is a token manager token.
    Withdraw {},
    Receive(Cw20ReceiveMsg),
    // ROUTER MESSAGES
    /// Retrieves the desired `asset` and runs the `payload`, paying the required amount back to the vault
    /// after running the messages in the payload, and returning the profit to the sender.
    FlashLoan {
        asset: Asset,
        payload: Vec<CosmosMsg>,
    },
    /// Callback message for post-processing flash-loans.
    Callback(CallbackMsg),
    // /// Performs the next loan.
    // ///
    // /// Should only be called by internal contract.
    // NextLoan {
    //     /// The person to pay back all profits to
    //     initiator: Addr,
    //     /// The source vault's [AssetInfo]. Used for validation.
    //     source_vault_asset_info: AssetInfo,
    //     /// The final message to run once all assets have been loaned.
    //     payload: Vec<CosmosMsg>,
    //     /// The next loans to run.
    //     to_loan: Vec<(String, Asset)>,
    //     /// The assets that have been loaned
    //     loaned_assets: Vec<(String, Asset)>,
    // },
    // /// Completes the flash-loan by paying back all outstanding loans, and returning profits to the sender.
    // ///
    // /// Should only be called by internal contract.
    // CompleteLoan {
    //     /// The person to pay back all profits to
    //     initiator: Addr,
    //     /// A vec of tuples where the first value represents the vault address, and the second value represents the loan size
    //     loaned_assets: Vec<(String, Asset)>,
    // },
}

/// The migrate message
#[cw_serde]
pub struct MigrateMsg {}

/// The query message
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Retrieves the configuration of the vault.
    #[returns(ManagerConfig)]
    ManagerConfig {},
    /// Retrieves a vault given the asset_info.
    #[returns(VaultsResponse)]
    Vault { asset_info: AssetInfo },
    /// Retrieves the addresses for all the vaults.
    #[returns(VaultsResponse)]
    Vaults {
        start_after: Option<Vec<u8>>,
        limit: Option<u32>,
    },

    /// Retrieves the share of the assets stored in the vault that a given `lp_share` is entitled to.
    #[returns(ShareResponse)]
    Share { lp_share: Asset },
    /// Retrieves the [`Uint128`] amount that must be sent back to the contract to pay off a loan taken out.
    #[returns(PaybackAssetResponse)]
    PaybackAmount { asset: Asset },
}

/// Response for the vaults query
#[cw_serde]
pub struct VaultsResponse {
    pub vaults: Vec<Vault>,
}

/// The callback messages available. Only callable by the vault contract itself.
#[cw_serde]
pub enum CallbackMsg {
    AfterFlashloan {
        old_asset_balance: Uint128,
        loan_asset: Asset,
        sender: Addr,
    },
}

#[cw_serde]
pub enum Cw20HookMsg {
    /// Withdraws a given amount from the vault.
    Withdraw {},
}

#[cw_serde]
pub struct Cw20ReceiveMsg {
    pub sender: String,
    pub amount: Uint128,
    pub msg: Binary,
}

#[cw_serde]
pub struct PaybackAssetResponse {
    pub asset_info: AssetInfo,
    /// The total amount that must be returned. Equivalent to `amount` + `protocol_fee` + `flash_loan_fee`.
    pub payback_amount: Uint128,
    /// The amount of fee paid to the protocol
    pub protocol_fee: Uint128,
    /// The amount of fee paid to vault holders
    pub flash_loan_fee: Uint128,
}
#[cw_serde]
pub struct ShareResponse {
    /// The amount of assets that the given `lp_share` is entitled to.
    pub share: Asset,
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
