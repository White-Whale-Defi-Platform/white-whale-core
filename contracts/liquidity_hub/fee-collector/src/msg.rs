use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terraswap::asset::AssetInfo;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Adds a factory to the fee collector so it can be queried when collecting fees
    AddFactory { factory_addr: String },
    /// Removes a factory from the fee collector
    RemoveFactory { factory_addr: String },
    /// Collects all the fees accrued by the children of the registered factories. If a factory is
    /// provided, only the fees of that factory's children will be collected.
    CollectFees {
        factory_addr: Option<String>,
        contracts: Option<Vec<String>>,
        start_after: Option<[AssetInfo; 2]>,
        limit: Option<u32>,
    },
    /// Updates the config
    UpdateConfig { owner: Option<String> },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Queries factories added to the fee collector
    Factories {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Queries the configuration of this contract
    Config {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct FactoriesResponse {
    pub factories: Vec<Addr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
