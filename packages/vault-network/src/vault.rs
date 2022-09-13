use cosmwasm_std::{Addr, Binary, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terraswap::asset::{Asset, AssetInfo};
use white_whale::fee::VaultFee;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
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
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CallbackMsg {
    AfterTrade {
        old_balance: Uint128,
        loan_amount: Uint128,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Withdraws a given amount from the vault.
    Withdraw {},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Cw20ReceiveMsg {
    pub sender: String,
    pub amount: Uint128,
    pub msg: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Retrieves the configuration of the contract in a [`Config`] response.
    Config {},
    /// Retrieves the share of the assets stored in the vault that a given `amount` of lp tokens is entitled to in a [`Uint128`] response.
    Share { amount: Uint128 },
    /// Retrieves the protocol fees that have been collected. If `all_time` is `true`, will return the all time collected fees.
    ProtocolFees { all_time: bool },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

/// The `reply` code ID for the submessage after instantiating the LP token.
pub const INSTANTIATE_LP_TOKEN_REPLY_ID: u64 = 1;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ProtocolFeesResponse {
    pub fees: Asset,
}

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
