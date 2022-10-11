use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::asset::{AssetInfo, PairInfo};
use crate::pair::PoolFee;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Pair contract code ID, which is used to
    pub pair_code_id: u64,
    pub token_code_id: u64,
    pub fee_collector_addr: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Updates contract's config, i.e. relevant code_ids, fee_collector address and owner
    UpdateConfig {
        owner: Option<String>,
        fee_collector_addr: Option<String>,
        token_code_id: Option<u64>,
        pair_code_id: Option<u64>,
    },
    /// Instantiates pair contract
    CreatePair {
        /// Asset infos
        asset_infos: [AssetInfo; 2],
        pool_fees: PoolFee,
    },
    /// Adds native token info to the contract so it can instantiate pair contracts that include it
    AddNativeTokenDecimals { denom: String, decimals: u8 },
    /// Migrates a pair contract to a given code_id
    MigratePair {
        contract: String,
        code_id: Option<u64>,
    },
    /// Removes pair
    RemovePair { pair_address: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    Pair {
        asset_infos: [AssetInfo; 2],
    },
    Pairs {
        start_after: Option<[AssetInfo; 2]>,
        limit: Option<u32>,
    },
    NativeTokenDecimals {
        denom: String,
    },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
    pub fee_collector_addr: String,
    pub pair_code_id: u64,
    pub token_code_id: u64,
}

/// We currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PairsResponse {
    pub pairs: Vec<PairInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct NativeTokenDecimalsResponse {
    pub decimals: u8,
}
