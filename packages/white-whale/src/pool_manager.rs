use std::fmt;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Uint128};
use cw20::Cw20ReceiveMsg;
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use crate::pool_network::{
    asset::{Asset, AssetInfo, PairType},
    factory::NativeTokenDecimalsResponse,
    pair::{PoolFee, ReverseSimulationResponse, SimulationResponse},
    router::{SimulateSwapOperationsResponse},
};


#[cw_serde]
pub enum Cw20HookMsg {
    /// Sell a given amount of asset
    Swap {
        ask_asset: AssetInfo,
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        to: Option<String>,
        pair_identifier: String,
    },
    /// Withdraws liquidity
    WithdrawLiquidity { pair_identifier: String },
}



#[cw_serde]
pub enum SwapOperation {
    WhaleSwap {
        token_in_info: AssetInfo,
        token_out_info: AssetInfo,
        pool_identifier: String,
    },
}

impl SwapOperation {
    pub fn get_target_asset_info(&self) -> AssetInfo {
        match self {
            SwapOperation::WhaleSwap { token_out_info, .. } => token_out_info.clone(),
        }
    }
}

impl fmt::Display for SwapOperation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SwapOperation::WhaleSwap {
                token_in_info,
                token_out_info,
                pool_identifier,
            } => write!(
                f,
                "WhaleSwap {{ token_in_info: {token_in_info}, token_out_info: {token_out_info}, pool_identifier: {pool_identifier} }}"
            ),
            
        }
    }
}

#[cw_serde]
pub struct SwapRoute {
    pub offer_asset_info: AssetInfo,
    pub ask_asset_info: AssetInfo,
    pub swap_operations: Vec<SwapOperation>,
}

// Used for all swap routes
#[cw_serde]
pub struct SwapRouteResponse {
    pub offer_asset: String,
    pub ask_asset: String,
    pub swap_route: Vec<SwapOperation>,
}

impl fmt::Display for SwapRoute {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "SwapRoute {{ offer_asset_info: {}, ask_asset_info: {}, swap_operations: {:?} }}",
            self.offer_asset_info, self.ask_asset_info, self.swap_operations
        )
    }
}


// Define a structure for Fees which names a number of defined fee collection types, maybe leaving room for a custom room a user can use to pass a fee with a defined custom name
#[cw_serde]
pub enum Fee {
    Protocol,
    LiquidityProvider,
    FlashLoanFees,
    Custom(String),
}

// Store PairInfo to N
// We define a custom struct for which allows for dynamic but defined pairs
#[cw_serde]
pub struct NPairInfo {
    pub asset_infos: Vec<AssetInfo>,
    pub liquidity_token: AssetInfo,
    pub asset_decimals: Vec<u8>,
    pub balances: Vec<Uint128>,
    pub assets: Vec<Asset>,
    pub pair_type: PairType,
    pub pool_fees: PoolFee,
}
impl NPairInfo {}


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
        asset_infos: Vec<AssetInfo>,
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
    AddNativeTokenDecimals {
        denom: String,
        decimals: u8,
    },

    /// Execute multiple [SwapOperation]s, i.e. multi-hop swaps.
    ExecuteSwapOperations {
        operations: Vec<SwapOperation>,
        minimum_receive: Option<Uint128>,
        to: Option<String>,
        max_spread: Option<Decimal>,
    },
    /// Swap the offer to ask token. This message can only be called internally by the router contract.
    ExecuteSwapOperation {
        operation: SwapOperation,
        to: Option<String>,
        max_spread: Option<Decimal>,
    },
    /// Checks if the swap amount exceeds the minimum_receive. This message can only be called
    /// internally by the router contract.
    AssertMinimumReceive {
        asset_info: AssetInfo,
        prev_balance: Uint128,
        minimum_receive: Uint128,
        receiver: String,
    },
    /// Adds swap routes to the router.
    AddSwapRoutes {
        swap_routes: Vec<SwapRoute>,
    }, 
    // CW20 Methods
    Receive(Cw20ReceiveMsg),
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
        pair_identifier: String,
    },
    /// Simulates a reverse swap, i.e. given the ask asset, how much of the offer asset is needed to
    /// perform the swap.
    #[returns(ReverseSimulationResponse)]
    ReverseSimulation {
        ask_asset: Asset,
        offer_asset: Asset,
        pair_identifier: String,
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

    #[returns(NPairInfo)]
    Pair { pair_identifier: String },
}
