use cosmwasm_std::{Binary, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terraswap::asset::AssetInfo;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// The owner of the contract.
    pub owner: String,
    /// The asset info the vault should manage.
    pub asset_info: AssetInfo,
    /// The code ID of the liquidity token to instantiate
    pub token_id: u64,
}

/// The callback messages available. Only callable by the vault contract itself.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CallbackMsg {
    AfterTrade { old_balance: Uint128 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Withdraws a given amount from the vault.
    Withdraw {},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Cw20ReceiveMsg {
    pub sender: String,
    pub amount: Uint128,
    pub msg: Binary,
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
    /// Updates the configuration of the contract.
    /// If a field is not specified, it will not be modified.
    UpdateConfig {
        /// If users should be allowed to perform flash-loans.
        flash_loan_enabled: Option<bool>,
        /// If users should be able to deposit funds to the contract.
        deposit_enabled: Option<bool>,
        /// if users should be able to withdraw funds from the contract.
        withdraw_enabled: Option<bool>,
        /// The new owner of the contract.
        new_owner: Option<String>,
    },
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
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

/// The `reply` code ID for the submessage after instantiating the LP token.
pub const INSTANTIATE_LP_TOKEN_REPLY_ID: u64 = 1;
