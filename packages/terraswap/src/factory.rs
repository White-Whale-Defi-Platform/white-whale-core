use cosmwasm_schema::{cw_serde, QueryResponses};

use crate::asset::{AssetInfo, PairInfo, PairType};
use crate::pair::{FeatureToggle, PoolFee};

#[cw_serde]
pub struct InstantiateMsg {
    /// Pair contract code ID, which is used to
    pub pair_code_id: u64,
    pub token_code_id: u64,
    pub fee_collector_addr: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Updates contract's config, i.e. relevant code_ids, fee_collector address and owner
    UpdateConfig {
        owner: Option<String>,
        fee_collector_addr: Option<String>,
        token_code_id: Option<u64>,
        pair_code_id: Option<u64>,
    },
    /// Updates a pair config
    UpdatePairConfig {
        pair_addr: String,
        owner: Option<String>,
        fee_collector_addr: Option<String>,
        pool_fees: Option<PoolFee>,
        feature_toggle: Option<FeatureToggle>,
    },
    /// Instantiates pair contract
    CreatePair {
        /// Asset infos
        asset_infos: [AssetInfo; 2],
        pool_fees: PoolFee,
        /// The variant of pair to create
        pair_type: PairType,
    },
    /// Adds native token info to the contract so it can instantiate pair contracts that include it
    AddNativeTokenDecimals { denom: String, decimals: u8 },
    /// Migrates a pair contract to a given code_id
    MigratePair {
        contract: String,
        code_id: Option<u64>,
    },
    /// Removes pair contract given asset infos
    RemovePair { asset_infos: [AssetInfo; 2] },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Retrieves the configuration of the factory.
    #[returns(ConfigResponse)]
    Config {},
    /// Retrieves the info for the pair with the given asset_infos.
    #[returns(PairInfo)]
    Pair { asset_infos: [AssetInfo; 2] },
    /// Retrieves the pairs created by the factory. This query has pagination enabled, querying ten
    /// items by default if not specified otherwise. The max amount of items that can be queried at
    /// once is 30. `start_after` is the last asset_info of a page.
    #[returns(PairsResponse)]
    Pairs {
        start_after: Option<[AssetInfo; 2]>,
        limit: Option<u32>,
    },
    /// Retrieves the decimals for the given native or ibc denom.
    #[returns(NativeTokenDecimalsResponse)]
    NativeTokenDecimals { denom: String },
}

// We define a custom struct for each query response
#[cw_serde]
pub struct ConfigResponse {
    pub owner: String,
    pub fee_collector_addr: String,
    pub pair_code_id: u64,
    pub token_code_id: u64,
}

/// We currently take no arguments for migrations
#[cw_serde]
pub struct MigrateMsg {}

// We define a custom struct for each query response
#[cw_serde]
pub struct PairsResponse {
    pub pairs: Vec<PairInfo>,
}

#[cw_serde]
pub struct NativeTokenDecimalsResponse {
    pub decimals: u8,
}
