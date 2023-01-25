use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub struct Config {
    /// Owner of the contract.
    pub owner: Addr,
    /// Unstaking period in number of blocks
    pub unstaking_period: u64,
    /// A scalar that controls the effect of time on the weight of a stake. If the growth rate is set
    /// to zero, time will have no impact on the weight. If the growth rate is set to one, the stake's
    /// weight will increase by one for each block.
    pub growth_rate: u8,
}

#[cw_serde]
pub struct InstantiateMsg {
    /// Unstaking period in number of blocks
    pub unstaking_period: u64,
    /// Weight grow rate
    pub growth_rate: u8,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Stakes the specified amount of tokens.
    Stake { amount: Uint128 },
    /// Unstakes the specified amount of tokens.
    Unstake { amount: Uint128 },
    /// Sends claimable unstaked tokens to the user.
    Claim {},
    /// Updates the [Config] of the contract.
    UpdateConfig {
        owner: Option<Addr>,
        unstaking_period: Option<u64>,
        growth_rate: Option<u8>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the [Config] of te contract.
    #[returns(Config)]
    Config {},

    /// Returns the amount of tokens that have been staked by the specified address.
    #[returns(Uint128)]
    Staked { address: String },

    /// Returns the amount of tokens that are been unstaked by the specified address.
    #[returns(Uint128)]
    Unstaking { address: String },

    /// Returns the amount of unstaking tokens of the specified address that can be claimed, i.e.
    /// that have passed the unstaking period.
    #[returns(Uint128)]
    Claimable { address: String },

    /// Returns the weight of the address.
    #[returns(StakingWeightResponse)]
    Weight { address: String },
}

/// Response for the vaults query
#[cw_serde]
pub struct StakingWeightResponse {
    pub address: String,
    pub weight: Uint128,
    pub global_weight: Uint128,
    pub share: Uint128,
}
