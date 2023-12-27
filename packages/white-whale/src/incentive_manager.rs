use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use std::collections::{BTreeMap, HashMap};

use crate::pool_network::asset::{Asset, AssetInfo};

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
    pub create_incentive_fee: Asset,
    /// The maximum amount of incentives that can exist for a single LP token at a time.
    pub max_concurrent_incentives: u32,
    /// New incentives are allowed to start up to `current_epoch + start_epoch_buffer` into the future.
    pub max_incentive_epoch_buffer: u32,
    /// The minimum amount of time that a user can bond their tokens for. In nanoseconds.
    pub min_unbonding_duration: u64,
    /// The maximum amount of time that a user can bond their tokens for. In nanoseconds.
    pub max_unbonding_duration: u64,
}

/// The execution messages
#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    /// Creates a new incentive contract tied to the `lp_asset` specified.
    CreateIncentive { params: IncentiveParams },
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
    pub create_incentive_fee: Asset,
    /// The maximum amount of incentives that can exist for a single LP token at a time.
    pub max_concurrent_incentives: u32,
    /// The maximum amount of epochs in the future a new incentive is allowed to start in.
    pub max_incentive_epoch_buffer: u32,
    /// The minimum amount of time that a user can bond their tokens for. In nanoseconds.
    pub min_unbonding_duration: u64,
    /// The maximum amount of time that a user can bond their tokens for. In nanoseconds.
    pub max_unbonding_duration: u64,
}

/// Parameters for creating incentive
#[cw_serde]
pub struct IncentiveParams {
    /// The LP asset to create the incentive for.
    pub lp_asset: AssetInfo,
    /// The epoch at which the incentive will start. If unspecified, it will start at the
    /// current epoch.
    pub start_epoch: Option<u64>,
    /// The epoch at which the incentive should end. If unspecified, the incentive will default to end at
    /// 14 epochs from the current one.
    pub end_epoch: Option<u64>,
    /// The type of distribution curve. If unspecified, the distribution will be linear.
    pub curve: Option<Curve>,
    /// The asset to be distributed in this incentive.
    pub incentive_asset: Asset,
    /// If set, it  will be used to identify the incentive.
    pub incentive_indentifier: Option<String>,
}

/// Represents an incentive.
#[cw_serde]
pub struct Incentive {
    /// The ID of the incentive.
    pub incentive_identifier: String,
    /// The account which opened the incentive and can manage it.
    pub incentive_creator: Addr,
    /// The LP asset to create the incentive for.
    pub lp_asset: AssetInfo,
    /// The asset the incentive was created to distribute.
    pub incentive_asset: Asset,
    /// The amount of the `incentive_asset` that has been claimed so far.
    pub claimed_amount: Uint128,
    /// The type of curve the incentive has.
    pub curve: Curve,
    /// The epoch at which the incentive starts.
    pub start_epoch: u64,
    /// The epoch at which the incentive ends.
    pub end_epoch: u64,
    /// emitted tokens
    pub emitted_tokens: HashMap<u64, Uint128>,
    /// A map containing the amount of tokens it was expanded to at a given epoch. This is used
    /// to calculate the right amount of tokens to distribute at a given epoch when a incentive is expanded.
    pub asset_history: BTreeMap<u64, (Uint128, u64)>,
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
