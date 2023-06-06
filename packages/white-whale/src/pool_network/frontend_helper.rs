use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal};

use super::asset::Asset;

#[cw_serde]
pub struct InstantiateMsg {
    /// The address of the incentive factory.
    pub incentive_factory: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Deposits assets into a pair pool and opens or expands a position on the respective incentive
    /// contract with the received LP tokens.
    Deposit {
        /// The address of the pair to deposit.
        pair_address: String,
        /// The assets to deposit into the pair.
        assets: [Asset; 2],
        /// The
        slippage_tolerance: Option<Decimal>,
        /// The amount of time in seconds to unbond tokens for when incentivizing.
        unbonding_duration: u64,
    },
    /// Updates the configuration of the frontend helper.
    UpdateConfig {
        /// The new incentive_factory_addr.
        incentive_factory_addr: Option<String>,
        /// The new owner.
        owner: Option<String>,
    },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Retrieves the current contract configuration.
    #[returns(ConfigResponse)]
    Config {},
}

pub type ConfigResponse = Config;

/// Stores the configuration of the frontend helper.
#[cw_serde]
pub struct Config {
    /// The address of the incentive factory.
    pub incentive_factory_addr: Addr,
    /// The owner of the of the frontend helper.
    pub owner: Addr,
}

#[cw_serde]
pub struct TempState {
    /// The amount of time in seconds to unbond tokens for when incentivizing.
    pub unbonding_duration: u64,
    /// The person who is creating the position after depositing.
    pub receiver: Addr,
    /// The address that is being deposited to.
    pub pair_addr: Addr,
}
