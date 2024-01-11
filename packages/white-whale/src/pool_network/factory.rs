use cosmwasm_schema::{cw_serde, QueryResponses};

use crate::pool_network::asset::{AssetInfo, PairInfo, PairType, TrioInfo};
use crate::pool_network::pair::{FeatureToggle, PoolFee};
use crate::pool_network::trio::{
    FeatureToggle as TrioFeatureToggle, PoolFee as TrioPoolFee, RampAmp,
};

#[cw_serde]
pub struct InstantiateMsg {
    /// Pair contract code ID, which is used to
    pub pair_code_id: u64,
    /// trio code id used for 3 pool stable swap
    pub trio_code_id: u64,
    pub token_code_id: u64,
    pub fee_collector_addr: String,
    #[cfg(feature = "osmosis")]
    pub osmosis_fee_collector_addr: String
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Updates contract's config, i.e. relevant code_ids, fee_collector address and owner
    #[cfg(not(feature = "osmosis"))]
    UpdateConfig {
        owner: Option<String>,
        fee_collector_addr: Option<String>,
        token_code_id: Option<u64>,
        pair_code_id: Option<u64>,
        trio_code_id: Option<u64>,
    },
    /// Updates contract's config, i.e. relevant code_ids, fee_collector address and owner
    #[cfg(feature = "osmosis")]
    UpdateConfig {
        owner: Option<String>,
        fee_collector_addr: Option<String>,
        osmosis_fee_collector_addr: Option<String>,
        token_code_id: Option<u64>,
        pair_code_id: Option<u64>,
        trio_code_id: Option<u64>,
    },
    /// Updates a pair config
    #[cfg(not(feature = "osmosis"))]
    UpdatePairConfig {
        pair_addr: String,
        owner: Option<String>,
        fee_collector_addr: Option<String>,
        pool_fees: Option<PoolFee>,
        feature_toggle: Option<FeatureToggle>,
    },
    #[cfg(feature = "osmosis")]
    UpdatePairConfig {
        pair_addr: String,
        owner: Option<String>,
        fee_collector_addr: Option<String>,
        osmosis_fee_collector_addr: Option<String>,
        pool_fees: Option<PoolFee>,
        feature_toggle: Option<FeatureToggle>,
    },
    /// Updates a trio config
    #[cfg(not(feature = "osmosis"))]
    UpdateTrioConfig {
        trio_addr: String,
        owner: Option<String>,
        fee_collector_addr: Option<String>,
        pool_fees: Option<TrioPoolFee>,
        feature_toggle: Option<TrioFeatureToggle>,
        amp_factor: Option<RampAmp>,
    },
    /// Updates a trio config
    #[cfg(feature = "osmosis")]
    UpdateTrioConfig {
        trio_addr: String,
        owner: Option<String>,
        fee_collector_addr: Option<String>,
        osmosis_fee_collector_addr: Option<String>,
        pool_fees: Option<TrioPoolFee>,
        feature_toggle: Option<TrioFeatureToggle>,
        amp_factor: Option<RampAmp>,
    },
    /// Instantiates pair contract
    CreatePair {
        /// Asset infos
        asset_infos: [AssetInfo; 2],
        pool_fees: PoolFee,
        /// The variant of pair to create
        pair_type: PairType,
        /// If true, the pair will use the token factory to create the LP token. If false, it will
        /// use a cw20 token instead.
        token_factory_lp: bool,
    },
    /// Instantiates pair contract
    CreateTrio {
        /// Asset infos
        asset_infos: [AssetInfo; 3],
        pool_fees: TrioPoolFee,
        amp_factor: u64,
        /// If true, the pair will use the token factory to create the LP token. If false, it will
        /// use a cw20 token instead.
        token_factory_lp: bool,
    },
    /// Adds native token info to the contract so it can instantiate pair contracts that include it
    AddNativeTokenDecimals { denom: String, decimals: u8 },
    /// Migrates a pair contract to a given code_id
    MigratePair {
        contract: String,
        code_id: Option<u64>,
    },
    /// Migrates a trio contract to a given code_id
    MigrateTrio {
        contract: String,
        code_id: Option<u64>,
    },
    /// Removes pair contract given asset infos
    RemovePair { asset_infos: [AssetInfo; 2] },
    /// Removes trio contract given asset infos
    RemoveTrio { asset_infos: [AssetInfo; 3] },
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
    /// Retrieves the info for the trio with the given asset_infos.
    #[returns(TrioInfo)]
    Trio { asset_infos: [AssetInfo; 3] },
    /// Retrieves the trios created by the factory. This query has pagination enabled, querying ten
    /// items by default if not specified otherwise. The max amount of items that can be queried at
    /// once is 30. `start_after` is the last asset_info of a page.
    #[returns(TriosResponse)]
    Trios {
        start_after: Option<[AssetInfo; 3]>,
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
    #[cfg(feature = "osmosis")]
    pub osmosis_fee_collector_addr: String,
    pub pair_code_id: u64,
    pub trio_code_id: u64,
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
pub struct TriosResponse {
    pub trios: Vec<TrioInfo>,
}

#[cw_serde]
pub struct NativeTokenDecimalsResponse {
    pub decimals: u8,
}
