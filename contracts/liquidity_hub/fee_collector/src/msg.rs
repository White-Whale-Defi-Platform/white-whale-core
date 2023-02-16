use cosmwasm_schema::{cw_serde, QueryResponses};

use pool_network::asset::{Asset, AssetInfo};

use crate::state::ConfigResponse;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    /// Collects protocol fees based on the configuration indicated by [FeesFor]
    CollectFees { collect_fees_for: FeesFor },
    /// Swaps the assets (fees) sitting in the fee collector into the given [AssetInfo] if possible.
    /// A [SwapRoute] should be available at the router to be able to make the swaps.
    AggregateFees {
        asset_info: AssetInfo,
        aggregate_fees_for: FeesFor,
    },
    /// Updates the config
    UpdateConfig {
        owner: Option<String>,
        pool_router: Option<String>,
    },
}

#[cw_serde]
pub enum FeesFor {
    /// Refers to the fees on the given contracts
    Contracts { contracts: Vec<Contract> },
    /// Refers to the fees on the contracts the given factory created
    Factory {
        factory_addr: String,
        factory_type: FactoryType,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Queries the configuration of this contract
    #[returns(ConfigResponse)]
    Config {},
    /// Queries fees collected by a given factory's children or individual contracts
    #[returns(Vec<Asset>)]
    Fees {
        query_fees_for: FeesFor,
        all_time: Option<bool>,
    },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub enum FactoryType {
    /// Vault Factory
    Vault {
        start_after: Option<Vec<u8>>,
        limit: Option<u32>,
    },
    /// Pool Factory
    Pool {
        start_after: Option<[AssetInfo; 2]>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct Contract {
    pub address: String,
    pub contract_type: ContractType,
}

#[cw_serde]
pub enum ContractType {
    /// Vault contract type
    Vault {},
    /// Pool/Pair contract type
    Pool {},
}
