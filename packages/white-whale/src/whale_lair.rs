use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub struct Config {
    /// Owner of the contract.
    pub owner: Addr,
    /// Unbonding period in number of blocks
    pub unbonding_period: u64,
    /// A scalar that controls the effect of time on the weight of a bond. If the growth rate is set
    /// to zero, time will have no impact on the weight. If the growth rate is set to one, the bond's
    /// weight will increase by one for each block.
    pub growth_rate: u8,
    /// Denom of the asset to be bonded. Can't only be set at instantiation.
    pub bonding_denom: String,
}

#[cw_serde]
#[derive(Default)]
pub struct Bond {
    /// The amount of bonded tokens.
    pub amount: Uint128,
    /// The block height at which the bond was done.
    pub block_height: u64,
    /// The weight of the bond at the given block height.
    pub weight: Uint128,
}

impl Bond {
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
    /// The total amount of tokens bonded in the contract.
    pub bond: Uint128,
    /// The block height at which the total bond was registered.
    pub block_height: u64,
    /// The total weight of the bond at the given block height.
    pub weight: Uint128,
}

#[cw_serde]
pub struct InstantiateMsg {
    /// Unbonding period in number of blocks
    pub unbonding_period: u64,
    /// Weight grow rate
    pub growth_rate: u8,
    /// Denom of the asset to be bonded
    pub bonding_denom: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Bonds the specified amount of tokens.
    Bond { amount: Uint128 },
    /// Unbonds the specified amount of tokens.
    Unbond { amount: Uint128 },
    /// Sends withdrawable unbonded tokens to the user.
    Withdraw {},
    /// Updates the [Config] of the contract.
    UpdateConfig {
        owner: Option<String>,
        unbonding_period: Option<u64>,
        growth_rate: Option<u8>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the [Config] of te contract.
    #[returns(Config)]
    Config {},

    /// Returns the amount of tokens that have been bonded by the specified address.
    #[returns(BondedResponse)]
    Bonded { address: String },

    /// Returns the amount of tokens that are been unbonded by the specified address.
    /// Allows pagination with start_after and limit.
    #[returns(UnbondingResponse)]
    Unbonding {
        address: String,
        start_after: Option<u64>,
        limit: Option<u8>,
    },

    /// Returns the amount of unbonding tokens of the specified address that can be withdrawn, i.e.
    /// that have passed the unbonding period.
    #[returns(WithdrawableResponse)]
    Withdrawable { address: String },

    /// Returns the weight of the address.
    #[returns(BondingWeightResponse)]
    Weight { address: String },
}

#[cw_serde]
pub struct MigrateMsg {}

/// Response for the Bonded query
#[cw_serde]
pub struct BondedResponse {
    pub bonded: Uint128,
}

/// Response for the Unstaking query
#[cw_serde]
pub struct UnbondingResponse {
    pub total_amount: Uint128,
    pub unbonding_requests: Vec<Bond>,
}

/// Response for the Withdrawable query
#[cw_serde]
pub struct WithdrawableResponse {
    pub claimable_amount: Uint128,
}

/// Response for the Weight query.
#[cw_serde]
pub struct BondingWeightResponse {
    pub address: String,
    pub weight: Uint128,
    pub global_weight: Uint128,
    pub share: Uint128,
}
