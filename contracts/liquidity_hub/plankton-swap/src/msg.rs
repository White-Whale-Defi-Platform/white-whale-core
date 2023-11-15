use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Uint128};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use white_whale::pool_network::{
    asset::{Asset, AssetInfo, PairType},
    factory::NativeTokenDecimalsResponse,
    pair::{PoolFee, ReverseSimulationResponse, SimulationResponse},
    router::{SimulateSwapOperationsResponse, SwapOperation, SwapRouteResponse},
};

use crate::state::NAssets;

#[cw_serde]
pub struct InstantiateMsg {
    pub fee_collector_addr: String,
    pub token_code_id: u64,
    pub pair_code_id: u64,
    pub owner: String,
    pub pool_creation_fee: Asset,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    CreatePair {
        asset_infos: NAssets,
        // TODO: Remap to NPoolFee maybe
        pool_fees: PoolFee,
        pair_type: PairType,
        token_factory_lp: bool,
        pair_identifier: Option<String>,
    },
    /// Provides liquidity to the pool
    ProvideLiquidity {
        assets: Vec<Asset>,
        slippage_tolerance: Option<Decimal>,
        receiver: Option<String>,
        pair_identifier: String,
    },
    /// Swap an offer asset to the other
    Swap {
        offer_asset: Asset,
        ask_asset: AssetInfo,
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        to: Option<String>,
        pair_identifier: String,
    },
    // /// Withdraws liquidity from the pool. Used only when the LP is a token factory token.
    WithdrawLiquidity {
        assets: Vec<Asset>,
        pair_identifier: String,
    },
    /// Adds native token info to the contract so it can instantiate pair contracts that include it
    AddNativeTokenDecimals { denom: String, decimals: u8 },
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Retrieves the decimals for the given native or ibc denom.
    #[returns(NativeTokenDecimalsResponse)]
    NativeTokenDecimals { denom: String },

    /// Simulates a swap.
    #[returns(SimulationResponse)]
    Simulation {
        offer_asset: Asset,
        ask_asset: Asset,
    },
    /// Simulates a reverse swap, i.e. given the ask asset, how much of the offer asset is needed to
    /// perform the swap.
    #[returns(ReverseSimulationResponse)]
    ReverseSimulation {
        ask_asset: Asset,
        offer_asset: Asset,
    },

    /// Gets the swap route for the given offer and ask assets.
    #[returns(Vec<SwapOperation>)]
    SwapRoute {
        offer_asset_info: AssetInfo,
        ask_asset_info: AssetInfo,
    },
    /// Gets all swap routes registered
    #[returns(Vec<SwapRouteResponse>)]
    SwapRoutes {},

    /// Simulates swap operations.
    #[returns(SimulateSwapOperationsResponse)]
    SimulateSwapOperations {
        offer_amount: Uint128,
        operations: Vec<SwapOperation>,
    },
    /// Simulates a reverse swap operations, i.e. given the ask asset, how much of the offer asset
    /// is needed to perform the swap.
    #[returns(SimulateSwapOperationsResponse)]
    ReverseSimulateSwapOperations {
        ask_amount: Uint128,
        operations: Vec<SwapOperation>,
    },
}
