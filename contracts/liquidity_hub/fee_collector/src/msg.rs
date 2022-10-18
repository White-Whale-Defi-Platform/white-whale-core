use cosmwasm_schema::{cw_serde, QueryResponses};

use terraswap::asset::{Asset, AssetInfo};

use crate::state::ConfigResponse;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    /// Collects protocol fees based on the configuration indicated by [CollectFeesFor]
    CollectFees { collect_fees_for: CollectFeesFor },
    /// Updates the config
    UpdateConfig { owner: Option<String> },
}

#[cw_serde]
pub enum CollectFeesFor {
    /// Collects the fees accumulated by the given contracts
    Contracts { contracts: Vec<Contract> },
    /// Collects the fees accumulated by the contracts the given factory created
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
        query_fees_for: QueryFeesFor,
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
pub enum QueryFeesFor {
    /// Specifies list of [Contract]s to query fees for
    Contracts { contracts: Vec<Contract> },
    /// Defines a factory for which to query fees from its children
    Factory {
        factory_addr: String,
        factory_type: FactoryType,
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
