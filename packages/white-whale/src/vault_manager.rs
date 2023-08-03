use crate::fee_distributor::Epoch;
use crate::pool_network::asset::{Asset, AssetInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, CosmosMsg, Uint128};
use crate::fee::VaultFee;
use crate::vault_network::vault;


/// The instantiation message
#[cw_serde]
pub struct InstantiateMsg {
    /// The owner of the manager
    pub owner: String,
    /// The code ID for the liquidity token contract in case the token manager is not used
    pub token_id: u64,
    /// The address where fees get collected
    pub fee_collector_addr: String,
    pub vault_creation_fee: Asset,
}

#[cw_serde]
pub struct ManagerConfig {
    pub owner: Addr,
    pub token_id: u64,
    pub fee_collector_addr: Addr,
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
    /// The asset info refenrece //TODO maybe this can be removed
    pub asset_info_reference: Vec<u8>,
    /// The LP asset
    pub lp_asset: AssetInfo,
    /// The fees associated with a vault
    pub fees: VaultFee,
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
        /// If true, the vault will use the token manager to create the LP token. If false, it will
        /// use a cw20 token instead.
        token_factory_lp: bool,
    },
    /// Removes a vault given its [AssetInfo]
    RemoveVault { asset_info: AssetInfo },
    /// Updates a vault config
    UpdateVault {
        vault_asset_info: AssetInfo,
        params: vault::UpdateConfigParams,
    },
    /// Updates the configuration of the vault manager.
    /// If a field is not specified, it will not be modified.
    UpdateManagerConfig {
        owner: Option<String>,
        fee_collector_addr: Option<String>,
        token_id: Option<u64>,
    },

    // VAULT MESSAGES

    /// Deposits a given asset into the vault manager.
    Deposit {
        asset: Asset,
    },
    /// Withdraws from the vault manager. Used when the LP token is a token manager token.
    Withdraw {},
    Receive(Cw20ReceiveMsg),
    Callback(CallbackMsg),

    // ROUTER MESSAGES

    /// Retrieves the desired `assets` and runs the `msgs`, paying the required amount back the vaults
    /// after running the messages, and returning the profit to the sender.
    FlashLoan {
        assets: Vec<Asset>,
        msgs: Vec<CosmosMsg>,
    },
    /// Performs the next loan.
    ///
    /// Should only be called by internal contract.
    NextLoan {
        /// The person to pay back all profits to
        initiator: Addr,
        /// The source vault's [AssetInfo]. Used for validation.
        source_vault_asset_info: AssetInfo,
        /// The final message to run once all assets have been loaned.
        payload: Vec<CosmosMsg>,
        /// The next loans to run.
        to_loan: Vec<(String, Asset)>,
        /// The assets that have been loaned
        loaned_assets: Vec<(String, Asset)>,
    },
    /// Completes the flash-loan by paying back all outstanding loans, and returning profits to the sender.
    ///
    /// Should only be called by internal contract.
    CompleteLoan {
        /// The person to pay back all profits to
        initiator: Addr,
        /// A vec of tuples where the first value represents the vault address, and the second value represents the loan size
        loaned_assets: Vec<(String, Asset)>,
    },
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
    #[returns(Option<Vault>)]
    Vault { asset_info: AssetInfo },
    /// Retrieves the addresses for all the vaults.
    #[returns(VaultsResponse)]
    Vaults {
        start_after: Option<Vec<u8>>,
        limit: Option<u32>,
    },

    /// Retrieves the share of the assets stored in the vault that a given `lp_share` is entitled to.
    #[returns(Asset)]
    Share { lp_share: Asset },
    /// Retrieves the protocol fees that have been collected.
    #[returns(ProtocolFeesResponse)]
    ProtocolFees {},
    /// Retrieves the fees that have been burned by the vault.
    #[returns(ProtocolFeesResponse)]
    BurnedFees {},
    /// Retrieves the [`Uint128`] amount that must be sent back to the contract to pay off a loan taken out.
    #[returns(PaybackAmountResponse)]
    GetPaybackAmount { asset: Asset },
}


/// Response for the vaults query
#[cw_serde]
pub struct VaultsResponse {
    pub vaults: Vec<Vault>,
}

/// The callback messages available. Only callable by the vault contract itself.
#[cw_serde]
pub enum CallbackMsg {
    AfterTrade {
        old_balance: Uint128,
        loan_amount: Uint128,
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
pub struct ProtocolFeesResponse {
    pub fees: Asset,
}

#[cw_serde]
pub struct PaybackAmountResponse {
    /// The total amount that must be returned. Equivalent to `amount` + `protocol_fee` + `flash_loan_fee`+ `burn_fee`.
    pub payback_amount: Asset,
    /// The amount of fee paid to the protocol
    pub protocol_fee: Asset,
    /// The amount of fee paid to vault holders
    pub flash_loan_fee: Asset,
    /// The amount of fee to be burned
    pub burn_fee: Asset,
}
