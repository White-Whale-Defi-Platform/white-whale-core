use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Uint128};
use pool_network::asset::{Asset, AssetInfo};
use white_whale::fee::VaultFee;

#[cw_serde]
pub struct InstantiateMsg {
    /// The owner of the contract.
    pub owner: String,
    /// The asset info the vault should manage.
    pub asset_info: AssetInfo,
    /// The code ID of the liquidity token to instantiate
    pub token_id: u64,
    /// The fees used for the vault
    pub vault_fees: VaultFee,
    /// The address of the fee collector
    pub fee_collector_addr: String,
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
pub struct UpdateConfigParams {
    /// If users should be allowed to perform flash-loans.
    pub flash_loan_enabled: Option<bool>,
    /// If users should be able to deposit funds to the contract.
    pub deposit_enabled: Option<bool>,
    /// if users should be able to withdraw funds from the contract.
    pub withdraw_enabled: Option<bool>,
    /// The new owner of the contract.
    pub new_owner: Option<String>,
    /// The new fees used for the vault
    pub new_vault_fees: Option<VaultFee>,
    /// The new address of the fee collector
    pub new_fee_collector_addr: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Deposits a given amount into the vault.
    Deposit {
        amount: Uint128,
    },
    /// Flash-loans a given amount from the vault.
    FlashLoan {
        amount: Uint128,
        msg: Binary,
    },
    /// Collects the Protocol fees
    CollectProtocolFees {},
    /// Updates the configuration of the contract.
    /// If a field is not specified, it will not be modified.
    UpdateConfig(UpdateConfigParams),
    Receive(Cw20ReceiveMsg),
    Callback(CallbackMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Retrieves the configuration of the contract.
    #[returns(Config)]
    Config {},
    /// Retrieves the share of the assets stored in the vault that a given `amount` of lp tokens is entitled to.
    #[returns(Uint128)]
    Share { amount: Uint128 },
    /// Retrieves the protocol fees that have been collected. If `all_time` is `true`, will return the all time collected fees.
    #[returns(ProtocolFeesResponse)]
    ProtocolFees { all_time: bool },
    /// Retrieves the fees that have been burned by the vault.
    #[returns(ProtocolFeesResponse)]
    BurnedFees {},
    /// Retrieves the [`Uint128`] amount that must be sent back to the contract to pay off a loan taken out.
    #[returns(PaybackAmountResponse)]
    GetPaybackAmount { amount: Uint128 },
}

#[cw_serde]
pub struct MigrateMsg {}

/// The `reply` code ID for the submessage after instantiating the LP token.
pub const INSTANTIATE_LP_TOKEN_REPLY_ID: u64 = 1;

#[cw_serde]
pub struct ProtocolFeesResponse {
    pub fees: Asset,
}

#[cw_serde]
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

#[cw_serde]
pub struct PaybackAmountResponse {
    /// The total amount that must be returned. Equivalent to `amount` + `protocol_fee` + `flash_loan_fee`+ `burn_fee`.
    pub payback_amount: Uint128,
    /// The amount of fee paid to the protocol
    pub protocol_fee: Uint128,
    /// The amount of fee paid to vault holders
    pub flash_loan_fee: Uint128,
    /// The amount of fee to be burned
    pub burn_fee: Uint128,
}
