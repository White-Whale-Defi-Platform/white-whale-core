use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use std::fmt;

use crate::pool_network::asset::{Asset, AssetInfo};

#[cw_serde]
pub struct InstantiateMsg {
    /// The address of the LP token that the incentive should be tied to.
    pub lp_asset: AssetInfo,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Opens a new liquidity flow
    OpenFlow {
        /// The start timestamp (in seconds since epoch) of the flow.
        ///
        /// If unspecified, the flow will start at the current block time.
        start_timestamp: Option<u64>,
        /// The timestamp (in seconds since epoch) the flow should end.
        end_timestamp: u64,
        /// The type of distribution curve.
        curve: Curve,
        /// The asset to be distributed in this flow.
        flow_asset: Asset,
    },
    /// Closes an existing liquidity flow.
    ///
    /// Sender of the message must either be the contract admin or the creator of the flow.
    CloseFlow {
        /// The id of the flow to close.
        flow_id: u64,
    },
    /// Creates a new position to earn flow rewards.
    OpenPosition {
        /// The amount to add to the position.
        amount: Uint128,
        /// The amount of time (in seconds) before the LP tokens can be redeemed.
        unbonding_duration: u64,
        /// The receiver of the new position.
        ///
        /// This is mostly used for the frontend helper contract.
        ///
        /// If left empty, defaults to the message sender.
        receiver: Option<String>,
    },
    ExpandPosition {
        /// The amount to add to the existing position.
        amount: Uint128,
        /// The unbond completion timestamp to identify the position to add to.
        unbonding_duration: u64,
        /// The receiver of the expanded position.
        ///
        /// This is mostly used for the frontend helper contract.
        ///
        /// If left empty, defaults to the message sender.
        receiver: Option<String>,
    },
    ClosePosition {
        /// The unbonding duration of the position to close.
        unbonding_duration: u64,
    },
    Withdraw {},
    Claim {},
}

#[cw_serde]
pub struct MigrateMsg {}

/// Represents a flow.
#[cw_serde]
pub struct Flow {
    /// A unique identifier of the flow.
    pub flow_id: u64,
    /// The account which opened the flow and can manage it.
    pub flow_creator: Addr,
    /// The asset the flow was created to distribute.
    pub flow_asset: Asset,
    /// The amount of the `flow_asset` that has been claimed so far.
    pub claimed_amount: Uint128,
    /// The type of curve the flow has.
    pub curve: Curve,
    /// The timestamp (in seconds block time) for when the flow began.
    pub start_timestamp: u64,
    /// The timestamp (in seconds block time) for when the flow will end.
    pub end_timestamp: u64,
}

/// Represents a position that accumulates flow rewards.
///
/// An address can have multiple incentive positions active at once.
#[cw_serde]
pub struct OpenPosition {
    /// The amount of LP tokens that are put up to earn incentives.
    pub amount: Uint128,
    /// Represents the amount of time in seconds the user must wait after unbonding for the LP tokens to be released.
    pub unbonding_duration: u64,
}

impl fmt::Display for OpenPosition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "OpenPosition(amount: {}, unbonding_duration: {})",
            self.amount, self.unbonding_duration
        )
    }
}

/// Represents a position that has moved from the [`OpenPosition`] state.
///
/// This position is no longer accumulating rewards, and the underlying tokens are claimable after `unbonding_duration`.
#[cw_serde]
pub struct ClosedPosition {
    /// The amount of LP tokens that the user is unbonding in this position.
    pub amount: Uint128,
    /// The block timestamp when the user can withdraw the position to retrieve the underlying `amount` of LP tokens.
    pub unbonding_timestamp: u64,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Retrieves the current contract configuration.
    #[returns(ConfigResponse)]
    Config {},
    /// Retrieves a specific flow.
    #[returns(FlowResponse)]
    Flow {
        /// The id of the flow to find.
        flow_id: u64,
    },
    /// Retrieves the current flows.
    #[returns(FlowsResponse)]
    Flows {},
    #[returns(PositionsResponse)]
    Positions {
        /// The address to get positions for.
        address: String,
    },
    #[returns(RewardsResponse)]
    Rewards {
        /// The address to get all the incentive rewards for.
        address: String,
    },
}

/// Stores the reply data set in the response when instantiating an incentive contract.
#[cw_serde]
pub struct InstantiateReplyCallback {
    /// The address of the LP token that is tied to the incentive contract.
    pub lp_asset: AssetInfo,
}

/// Represents the configuration of the incentive contract.
#[cw_serde]
pub struct Config {
    /// The address of the incentive factory.
    pub factory_address: Addr,

    /// The LP token asset tied to the incentive contract.
    pub lp_asset: AssetInfo,
}

/// The type of distribution curve to exist.
#[cw_serde]
pub enum Curve {
    /// A linear curve that releases assets as we approach the end of the flow period.
    Linear,
}

impl std::fmt::Display for Curve {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Curve::Linear => write!(f, "Linear"),
        }
    }
}

pub type ConfigResponse = Config;

#[cw_serde]
pub struct FlowResponse {
    //TODO why is this returning a Option<Flow>? why not a flow directly?
    /// The flow that was searched for.
    pub flow: Option<Flow>,
}

#[cw_serde]
pub struct FlowsResponse {
    /// The current flows.
    pub flows: Vec<Flow>,
}

#[cw_serde]
pub enum QueryPosition {
    /// Represents a position that a user has deposited, but not yet begun to unbond.
    OpenPosition {
        /// The amount of LP tokens the user deposited into the position.
        amount: Uint128,
        /// The amount of time (in seconds) the user must wait after they begin the unbonding process.
        unbonding_duration: u64,
        /// The amount of weight the position has.
        weight: Uint128,
    },
    /// Represents a position that a user has initiated the unbonding process on. The position may or may not be withdrawable.
    ClosedPosition {
        /// The amount of LP tokens the user deposited into the position, and will receive after they withdraw.
        amount: Uint128,
        /// The timestamp (in seconds) the user unbonded at.
        unbonding_timestamp: u64,
        /// The amount of weight the position has.
        weight: Uint128,
    },
}

#[cw_serde]
pub struct PositionsResponse {
    /// The current time of the blockchain.
    pub timestamp: u64,
    /// All the positions a user has.
    pub positions: Vec<QueryPosition>,
}

#[cw_serde]
pub struct RewardsResponse {
    /// The rewards that is available to a user if they executed the `claim` function at this point.
    pub rewards: Vec<Asset>,
}
