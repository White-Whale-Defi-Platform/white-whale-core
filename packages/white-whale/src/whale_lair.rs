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
    /// Denom of the asset to be staked. Can't only be set at instantiation.
    pub staking_denom: String,
}

#[cw_serde]
#[derive(Default)]
pub struct Stake {
    /// The amount of staked tokens.
    pub amount: Uint128,
    /// The block height at which the stake was done.
    pub block_height: u64,
    /// The weight of the stake at the given block height.
    pub weight: Uint128,
}

impl Stake {
    pub fn default() -> Self {
        Self {
            amount: Uint128::zero(),
            block_height: 0u64,
            weight: Uint128::zero(),
        }
    }
}

#[cw_serde]
#[derive(Default)]
pub struct GlobalIndex {
    /// The total amount of tokens staked in the contract.
    pub stake: Uint128,
    /// The block height at which the total stake was registered.
    pub block_height: u64,
    /// The total weight of the stake at the given block height.
    pub weight: Uint128,
}

#[cw_serde]
pub struct InstantiateMsg {
    /// Unstaking period in number of blocks
    pub unstaking_period: u64,
    /// Weight grow rate
    pub growth_rate: u8,
    /// Denom of the asset to be staked
    pub staking_denom: String,
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
        owner: Option<String>,
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
    #[returns(StakedResponse)]
    Staked { address: String },

    /// Returns the amount of tokens that are been unstaked by the specified address.
    /// Allows pagination with start_after and limit.
    #[returns(UnstakingResponse)]
    Unstaking {
        address: String,
        start_after: Option<u64>,
        limit: Option<u8>,
    },

    /// Returns the amount of unstaking tokens of the specified address that can be claimed, i.e.
    /// that have passed the unstaking period.
    #[returns(ClaimableResponse)]
    Claimable { address: String },

    /// Returns the weight of the address.
    #[returns(StakingWeightResponse)]
    Weight { address: String },
}

#[cw_serde]
pub struct MigrateMsg {}

/// Response for the Staked query
#[cw_serde]
pub struct StakedResponse {
    pub staked: Uint128,
}

/// Response for the Unstaking query
#[cw_serde]
pub struct UnstakingResponse {
    pub total_amount: Uint128,
    pub unstaking_requests: Vec<Stake>,
}

/// Response for the Claimable query
#[cw_serde]
pub struct ClaimableResponse {
    pub claimable_amount: Uint128,
}

/// Response for the Weight query.
#[cw_serde]
pub struct StakingWeightResponse {
    pub address: String,
    pub weight: Uint128,
    pub global_weight: Uint128,
    pub share: Uint128,
}
