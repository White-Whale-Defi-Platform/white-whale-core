use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

use crate::pool_network::asset::{Asset, AssetInfo};

#[cw_serde]
pub struct InstantiateMsg {
    /// The address of the fee collector to send flow creation fees to.
    pub fee_collector_addr: String,
    /// Fee distributor contract address.
    pub fee_distributor_addr: String,
    /// The fee that must be paid to create a flow.
    pub create_flow_fee: Asset,
    /// The maximum amount of flows that can exist for a single LP token at a single time.
    pub max_concurrent_flows: u64,
    /// The code ID of the incentive contract.
    pub incentive_code_id: u64,
    /// The maximum epoch buffer for a new flow (in epochs).
    ///
    /// New flows are allowed to start up to `current_epoch + start_epoch_buffer` into the future.
    pub max_flow_epoch_buffer: u64,
    /// The minimum amount of seconds that a user must bond their tokens for.
    pub min_unbonding_duration: u64,
    /// The maximum amount of seconds that a user must bond their tokens for.
    pub max_unbonding_duration: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Creates a new incentive contract tied to the `lp_asset` specified.
    CreateIncentive { lp_asset: AssetInfo },
    /// Updates the configuration of the contract.
    ///
    /// Unspecified fields will not be updated.
    UpdateConfig {
        /// The owner of the contract.
        ///
        /// If unspecified, the owner address will not change.
        owner: Option<String>,
        /// The new fee collector address to send flow creation fees to.
        ///
        /// If unspecified, the fee collector address will not change.
        fee_collector_addr: Option<String>,
        /// The new fee distributor address to get epochs from.
        ///
        /// If unspecified, the fee distributor address will not change.
        fee_distributor_addr: Option<String>,
        /// The new fee that must be paid to create a flow.
        ///
        /// If unspecified, the flow fee will not change.
        create_flow_fee: Option<Asset>,
        /// The maximum amount of concurrent flows that can exist for a single LP token at a single time.
        ///
        /// If unspecified, the max concurrent flows will not change.
        max_concurrent_flows: Option<u64>,
        /// The new code ID of the incentive contract.
        ///
        /// If unspecified, the incentive contract id will not change.
        incentive_code_id: Option<u64>,

        /// The new maximum start time buffer for a new flow (in seconds).
        ///
        /// If unspecified, the flow start buffer will not change.
        max_flow_start_time_buffer: Option<u64>,
        /// The minimum amount of seconds that a user must bond their tokens for.
        ///
        /// If unspecified, the `min_unbonding_duration` will not change.
        min_unbonding_duration: Option<u64>,
        /// The maximum amount of seconds that a user must bond their tokens for.
        ///
        /// If unspecified, the `max_unbonding_duration` will not change.
        max_unbonding_duration: Option<u64>,
    },
    MigrateIncentive {
        /// The address of the incentive contract
        incentive_address: String,
        /// The new code ID to migrate the incentive contract to.
        ///
        /// If unspecified, will default to the incentive factory's configured incentive code ID.
        code_id: Option<u64>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Retrieves the config of the incentive factory.
    #[returns(ConfigResponse)]
    Config {},
    /// Retrieves a specific incentive address.
    #[returns(IncentiveResponse)]
    Incentive {
        /// The LP token asset info.
        lp_asset: AssetInfo,
    },
    #[returns(IncentivesResponse)]
    Incentives {
        /// An optional parameter specifying what incentive contract to start
        /// searching after.
        start_after: Option<AssetInfo>,
        /// The amount of incentive contracts to return.
        ///
        /// If unspecified, will default to a value specified by the contract.
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct MigrateMsg {}

/// Stores the configuration of the incentive factory.
#[cw_serde]
pub struct Config {
    pub owner: Addr,
    /// The address to send fees to.
    pub fee_collector_addr: Addr,
    /// Fee distributor contract address.
    pub fee_distributor_addr: Addr,
    /// The fee that must be paid each time a user wants to create a flow.
    pub create_flow_fee: Asset,
    /// The maximum amount of flows that can exist at any one time.
    pub max_concurrent_flows: u64,
    /// The code ID of the incentive contract.
    pub incentive_code_id: u64,
    /// The maximum amount of epochs in the future a new flow is allowed to start in.
    pub max_flow_epoch_buffer: u64,
    /// The minimum amount of seconds that a user must bond their tokens for.
    pub min_unbonding_duration: u64,
    /// The maximum amount of seconds that a user must bond their tokens for.
    pub max_unbonding_duration: u64,
}

pub type ConfigResponse = Config;
pub type IncentiveResponse = Option<Addr>;

#[cw_serde]
pub struct IncentivesContract {
    /// The address of the incentive contract.
    pub incentive_address: Addr,
    /// A byte-array reference to the LP address.
    pub lp_reference: Vec<u8>,
}
pub type IncentivesResponse = Vec<IncentivesContract>;
