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
}

/// The callback messages available. Only callable by the vault contract itself.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum CallbackMsg {
    AfterTrade { old_balance: Uint128 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ExecuteMsg {
    /// Deposits a given amount into the vault.
    Deposit {
        amount: Uint128,
    },
    /// Withdraws a given amount from the vault.
    Withdraw {
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
    Callback(CallbackMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum QueryMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
