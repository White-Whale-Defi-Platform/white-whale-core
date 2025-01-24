use crate::fee_distributor::Epoch;
use crate::pool_network::asset::{Asset, AssetInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Decimal, Uint64};

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    /// Collects protocol fees based on the configuration indicated by [FeesFor]
    CollectFees { collect_fees_for: FeesFor },
    /// Swaps the assets (fees) sitting in the fee collector into the distribution asset set by the
    /// fee collector. A [SwapRoute] should be available at the router to be able to make the swaps.
    AggregateFees { aggregate_fees_for: FeesFor },
    /// Forward fees to the fee distributor. This will collect and aggregate the fees, to send them back to the fee distributor.
    ForwardFees {
        epoch: Epoch,
        forward_fees_as: AssetInfo,
    },
    /// Updates the config
    UpdateConfig {
        owner: Option<String>,
        pool_router: Option<String>,
        fee_distributor: Option<String>,
        pool_factory: Option<String>,
        vault_factory: Option<String>,
        take_rate: Option<Decimal>,
        take_rate_dao_address: Option<String>,
        is_take_rate_active: Option<bool>,
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
    #[returns(Config)]
    Config {},
    /// Queries fees collected by a given factory's children or individual contracts
    #[returns(Vec<Asset>)]
    Fees {
        query_fees_for: FeesFor,
        all_time: Option<bool>,
    },
    /// Queries the take rate taken for the given epoch id.
    #[returns(Coin)]
    TakeRateHistory { epoch_id: Uint64 },
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

#[cw_serde]
pub struct ForwardFeesResponse {
    pub epoch: Epoch,
}

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub pool_router: Addr,
    pub fee_distributor: Addr,
    pub pool_factory: Addr,
    pub vault_factory: Addr,
    pub take_rate: Decimal,
    pub take_rate_dao_address: Addr,
    pub is_take_rate_active: bool,
}
