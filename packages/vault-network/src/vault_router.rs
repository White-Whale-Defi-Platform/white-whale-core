use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, CosmosMsg};

use pool_network::asset::{Asset, AssetInfo};

/// The instantiation message
#[cw_serde]
pub struct InstantiateMsg {
    /// The owner of the router
    pub owner: String,
    /// The address for the vault factory
    pub vault_factory_addr: String,
}

/// The execution message
#[cw_serde]
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
        /// The vault contract that calls the [NextLoan] message
        source_vault: String,
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

/// The query message
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Retrieves the configuration of the vault router.
    #[returns(Config)]
    Config {},
}

/// The migrate message
#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct Config {
    /// The owner of the router to update configuration
    pub owner: Addr,
    /// The address of the vault factory
    pub vault_factory: Addr,
}
