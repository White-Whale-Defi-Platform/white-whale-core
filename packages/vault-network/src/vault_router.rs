use cosmwasm_std::{Addr, CosmosMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use terraswap::asset::Asset;

/// The instantiation message
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// The owner of the router
    pub owner: String,
    /// The address for the vault factory
    pub vault_factory_addr: String,
}

/// The execution message
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Retrieves the desired `assets` and runs the `msgs`, paying the required amount back the vaults
    /// after running the messages, and returning the profit to the sender.
    FlashLoan {
        assets: Vec<Asset>,
        msgs: Vec<CosmosMsg>,
    },
    /// Updates the configuration of the vault router.
    ///
    /// If a field is not specified, it will not be modified.
    UpdateConfig {
        owner: Option<String>,
        vault_factory_addr: Option<String>,
    },
    /// Performs the next loan.
    ///
    /// Should only be called by internal contract.
    NextLoan {
        /// The person to pay back all profits to
        initiator: Addr,
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

/// The query message
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Retrieves the configuration of the vault router. Returns a [`Config`] struct.
    Config {},
}

/// The migrate message
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
