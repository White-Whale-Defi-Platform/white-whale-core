use cosmwasm_schema::cw_serde;
use cosmwasm_std::{CanonicalAddr, Decimal};

use super::asset::Asset;

#[cw_serde]
pub struct InstantiateMsg {
    /// The address of the incentive factory.
    pub incentive_factory: String,
}

#[cw_serde]
pub enum ExecuteMsg {
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
}

#[cw_serde]
pub struct MigrateMsg {}

/// Stores the configuration of the frontend helper.
#[cw_serde]
pub struct Config {
    /// The address of the incentive factory.
    pub incentive_factory_addr: CanonicalAddr,
}

#[cw_serde]
pub struct TempState {
    /// The amount of time in seconds to unbond tokens for when incentivizing.
    pub unbonding_duration: u64,
    /// The person who is creating the position after depositing.
    pub receiver: CanonicalAddr,
    /// The address that is being deposited to.
    pub pair_addr: CanonicalAddr,
}
