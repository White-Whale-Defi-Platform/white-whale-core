use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use std::collections::HashMap;

use crate::epoch_manager::hooks::EpochChangedHookMsg;

/// The instantiation message
#[cw_serde]
pub struct InstantiateMsg {
    /// The owner of the contract
    pub owner: String,
    /// The epoch manager address, where the epochs are managed
    pub epoch_manager_addr: String,
    /// The whale lair address, where protocol fees are distributed
    pub whale_lair_addr: String,
    /// The fee that must be paid to create an incentive.
    pub create_incentive_fee: Coin,
    /// The maximum amount of incentives that can exist for a single LP token at a time.
    pub max_concurrent_incentives: u32,
    /// New incentives are allowed to start up to `current_epoch + start_epoch_buffer` into the future.
    pub max_incentive_epoch_buffer: u32,
    /// The minimum amount of time that a user can lock their tokens for. In seconds.
    pub min_unlocking_duration: u64,
    /// The maximum amount of time that a user can lock their tokens for. In seconds.
    pub max_unlocking_duration: u64,
    /// The penalty for unlocking a position before the unlocking duration finishes. In percentage.
    pub emergency_unlock_penalty: Decimal,
}

/// The execution messages
#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    /// Manages an incentive based on the action, which can be:
    /// - Fill: Creates or expands an incentive.
    /// - Close: Closes an existing incentive.
    ManageIncentive { action: IncentiveAction },
    /// Manages a position based on the action, which can be:
    /// - Fill: Creates or expands a position.
    /// - Close: Closes an existing position.
    ManagePosition { action: PositionAction },
    /// Gets triggered by the epoch manager when a new epoch is created
    EpochChangedHook(EpochChangedHookMsg),
    /// Claims the rewards for the user
    Claim,
    /// Updates the config of the contract
    UpdateConfig {
        /// The address to of the whale lair, to send fees to.
        whale_lair_addr: Option<String>,
        /// The epoch manager address, where the epochs are managed
        epoch_manager_addr: Option<String>,
        /// The fee that must be paid to create an incentive.
        create_incentive_fee: Option<Coin>,
        /// The maximum amount of incentives that can exist for a single LP token at a time.
        max_concurrent_incentives: Option<u32>,
        /// The maximum amount of epochs in the future a new incentive is allowed to start in.
        max_incentive_epoch_buffer: Option<u32>,
        /// The minimum amount of time that a user can lock their tokens for. In seconds.
        min_unlocking_duration: Option<u64>,
        /// The maximum amount of time that a user can lock their tokens for. In seconds.
        max_unlocking_duration: Option<u64>,
        /// The penalty for unlocking a position before the unlocking duration finishes. In percentage.
        emergency_unlock_penalty: Option<Decimal>,
    },
}

/// The migrate message
#[cw_serde]
pub struct MigrateMsg {}

/// The query messages
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Retrieves the configuration of the manager.
    #[returns(Config)]
    Config {},
}

/// Configuration for the contract (manager)
#[cw_serde]
pub struct Config {
    /// The address to of the whale lair, to send fees to.
    pub whale_lair_addr: Addr,
    /// The epoch manager address, where the epochs are managed
    pub epoch_manager_addr: Addr,
    /// The fee that must be paid to create an incentive.
    pub create_incentive_fee: Coin,
    /// The maximum amount of incentives that can exist for a single LP token at a time.
    pub max_concurrent_incentives: u32,
    /// The maximum amount of epochs in the future a new incentive is allowed to start in.
    pub max_incentive_epoch_buffer: u32,
    /// The minimum amount of time that a user can lock their tokens for. In seconds.
    pub min_unlocking_duration: u64,
    /// The maximum amount of time that a user can lock their tokens for. In seconds.
    pub max_unlocking_duration: u64,
    /// The penalty for unlocking a position before the unlocking duration finishes. In percentage.
    pub emergency_unlock_penalty: Decimal,
}

/// Parameters for creating incentive
#[cw_serde]
pub struct IncentiveParams {
    /// The LP asset denom to create the incentive for.
    pub lp_denom: String,
    /// The epoch at which the incentive will start. If unspecified, it will start at the
    /// current epoch.
    pub start_epoch: Option<u64>,
    /// The epoch at which the incentive should preliminarily end (if it's not expanded). If
    /// unspecified, the incentive will default to end at 14 epochs from the current one.
    pub preliminary_end_epoch: Option<u64>,
    /// The type of distribution curve. If unspecified, the distribution will be linear.
    pub curve: Option<Curve>,
    /// The asset to be distributed in this incentive.
    pub incentive_asset: Coin,
    /// If set, it  will be used to identify the incentive.
    pub incentive_identifier: Option<String>,
}

#[cw_serde]
pub enum IncentiveAction {
    /// Fills an incentive. If the incentive doesn't exist, it creates a new one. If it exists already,
    /// it expands it given the sender created the original incentive and the params are correct.
    Fill {
        /// The parameters for the incentive to fill.
        params: IncentiveParams,
    },
    //// Closes an incentive with the given identifier. If the incentive has expired, anyone can
    // close it. Otherwise, only the incentive creator or the owner of the contract can close an incentive.
    Close {
        /// The incentive identifier to close.
        incentive_identifier: String,
    },
}

#[cw_serde]
pub enum PositionAction {
    /// Fills a position. If the position doesn't exist, it opens it. If it exists already,
    /// it expands it given the sender opened the original position and the params are correct.
    Fill {
        /// The identifier of the position.
        identifier: Option<String>,
        /// The time it takes in seconds to unlock this position. This is used to identify the position to fill.
        unlocking_duration: u64,
        /// The receiver for the position.
        /// If left empty, defaults to the message sender.
        receiver: Option<String>,
    },
    /// Closes an existing position. The position stops earning incentive rewards.
    Close {
        /// The identifier of the position.
        identifier: String,
        /// The asset to add to the position. If not set, the position will be closed in full. If not, it could be partially closed.
        lp_asset: Option<Coin>,
    },
    /// Withdraws the LP tokens from a position after the position has been closed and the unlocking duration has passed.
    Withdraw {
        /// The identifier of the position.
        identifier: String,
        /// Whether to unlock the position in an emergency. If set to true, the position will be unlocked immediately, but with a penalty.
        emergency_unlock: Option<bool>,
    },
}

// type for the epoch id
pub type EpochId = u64;

/// Represents an incentive.
#[cw_serde]
pub struct Incentive {
    /// The ID of the incentive.
    pub identifier: String,
    /// The account which opened the incentive and can manage it.
    pub owner: Addr,
    /// The LP asset denom to create the incentive for.
    pub lp_denom: String,
    /// The asset the incentive was created to distribute.
    pub incentive_asset: Coin,
    /// The amount of the `incentive_asset` that has been claimed so far.
    pub claimed_amount: Uint128,
    /// The amount of the `incentive_asset` that is to be distributed every epoch.
    pub emission_rate: Uint128,
    /// The type of curve the incentive has.
    pub curve: Curve,
    /// The epoch at which the incentive starts.
    pub start_epoch: EpochId,
    /// The epoch at which the incentive will preliminary end (in case it's not expanded).
    pub preliminary_end_epoch: EpochId,
    /// The last epoch this incentive was claimed.
    pub last_epoch_claimed: EpochId,
}

impl Incentive {
    /// Returns true if the incentive is expired
    pub fn is_expired(&self, epoch_id: EpochId) -> bool {
        self.incentive_asset
            .amount
            .saturating_sub(self.claimed_amount)
            < MIN_INCENTIVE_AMOUNT
            || epoch_id >= self.last_epoch_claimed + DEFAULT_INCENTIVE_DURATION
    }
}

#[cw_serde]
pub enum Curve {
    /// A linear curve that releases assets uniformly over time.
    Linear,
}

impl std::fmt::Display for Curve {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Curve::Linear => write!(f, "linear"),
        }
    }
}

/// Represents an LP position.
#[cw_serde]
pub struct Position {
    /// The identifier of the position.
    pub identifier: String,
    /// The amount of LP tokens that are put up to earn incentives.
    pub lp_asset: Coin,
    /// Represents the amount of time in seconds the user must wait after unlocking for the LP tokens to be released.
    pub unlocking_duration: u64,
    /// If true, the position is open. If false, the position is closed.
    pub open: bool,
    /// The block height at which the position, after being closed, can be withdrawn.
    pub expiring_at: Option<u64>,
    /// The owner of the position.
    pub receiver: Addr,
}
#[cw_serde]
pub enum RewardsResponse {
    RewardsResponse {
        /// The rewards that is available to a user if they executed the `claim` function at this point.
        rewards: Vec<Coin>,
    },
    ClaimRewards {
        /// The rewards that is available to a user if they executed the `claim` function at this point.
        rewards: Vec<Coin>,
        /// The rewards that were claimed on each incentive, if any.
        modified_incentives: HashMap<String, Uint128>,
    },
}

/// Minimum amount of an asset to create an incentive with
pub const MIN_INCENTIVE_AMOUNT: Uint128 = Uint128::new(1_000u128);

/// Default incentive duration in epochs
pub const DEFAULT_INCENTIVE_DURATION: u64 = 14u64;
