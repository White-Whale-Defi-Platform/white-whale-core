use cosmwasm_schema::{cw_serde, QueryResponses};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use crate::pool_network::asset::{Asset, AssetInfo};
use crate::vault_manager::LpTokenType;

/// The instantiation message
#[cw_serde]
pub struct InstantiateMsg {
    /// The owner of the contract
    pub owner: String,
    /// The whale lair address, where protocol fees are distributed
    pub whale_lair_addr: String,
    /// The fee that must be paid to create a flow.
    pub create_flow_fee: Asset,
    /// The maximum amount of flows that can exist for a single LP token at a time.
    pub max_concurrent_flows: u64,
    /// New flows are allowed to start up to `current_epoch + start_epoch_buffer` into the future.
    pub max_flow_epoch_buffer: u64,
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

pub struct Config {}

#[cw_serde]
pub struct IncentiveParams {
    lp_asset: AssetInfo,

}
