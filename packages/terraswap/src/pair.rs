use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;

use white_whale::fee::Fee;

use crate::asset::{Asset, AssetInfo, PairInfo};

#[cw_serde]
pub struct InstantiateMsg {
    /// Asset infos
    pub asset_infos: [AssetInfo; 2],
    /// Token contract code id for initialization
    pub token_code_id: u64,
    pub asset_decimals: [u8; 2],
    pub pool_fees: PoolFee,
    pub fee_collector_addr: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    /// ProvideLiquidity a user provides pool liquidity
    ProvideLiquidity {
        assets: [Asset; 2],
        slippage_tolerance: Option<Decimal>,
        receiver: Option<String>,
    },
    /// Swap an offer asset to the other
    Swap {
        offer_asset: Asset,
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        to: Option<String>,
    },
    /// Updates the pair pool config
    UpdateConfig {
        owner: Option<String>,
        fee_collector_addr: Option<String>,
        pool_fees: Option<PoolFee>,
        feature_toggle: Option<FeatureToggle>,
    },
    /// Collects the Protocol fees
    CollectProtocolFees {},
}

#[cw_serde]
pub enum Cw20HookMsg {
    /// Sell a given amount of asset
    Swap {
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        to: Option<String>,
    },
    WithdrawLiquidity {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(PairInfo)]
    Pair {},
    #[returns(ConfigResponse)]
    Config {},
    #[returns(ProtocolFeesResponse)]
    ProtocolFees {
        asset_id: Option<String>,
        all_time: Option<bool>,
    },
    #[returns(PoolResponse)]
    Pool {},
    #[returns(SimulationResponse)]
    Simulation { offer_asset: Asset },
    #[returns(ReverseSimulationResponse)]
    ReverseSimulation { ask_asset: Asset },
}

// Pool feature toggle
#[cw_serde]
pub struct FeatureToggle {
    pub withdrawals_enabled: bool,
    pub deposits_enabled: bool,
    pub swaps_enabled: bool,
}

/// Fees used by the pools on the pool network
#[cw_serde]
pub struct PoolFee {
    pub protocol_fee: Fee,
    pub swap_fee: Fee,
}

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub fee_collector_addr: Addr,
    pub pool_fees: PoolFee,
    pub feature_toggle: FeatureToggle,
}

pub type ConfigResponse = Config;

// We define a custom struct for each query response
#[cw_serde]
pub struct PoolResponse {
    pub assets: [Asset; 2],
    pub total_share: Uint128,
}

/// SimulationResponse returns swap simulation response
#[cw_serde]
pub struct SimulationResponse {
    pub return_amount: Uint128,
    pub spread_amount: Uint128,
    pub swap_fee_amount: Uint128,
    pub protocol_fee_amount: Uint128,
}

/// ReverseSimulationResponse returns reverse swap simulation response
#[cw_serde]
pub struct ProtocolFeesResponse {
    pub fees: Vec<Asset>,
}

/// ReverseSimulationResponse returns reverse swap simulation response
#[cw_serde]
pub struct ReverseSimulationResponse {
    pub offer_amount: Uint128,
    pub spread_amount: Uint128,
    pub swap_fee_amount: Uint128,
    pub protocol_fee_amount: Uint128,
}

/// We currently take no arguments for migrations
#[cw_serde]
pub struct MigrateMsg {}
