use std::fmt;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;

use crate::asset::AssetInfo;

#[cw_serde]
pub struct InstantiateMsg {
    pub terraswap_factory: String,
}

#[cw_serde]
pub enum SwapOperation {
    TerraSwap {
        offer_asset_info: AssetInfo,
        ask_asset_info: AssetInfo,
    },
}

impl SwapOperation {
    pub fn get_target_asset_info(&self) -> AssetInfo {
        match self {
            SwapOperation::TerraSwap { ask_asset_info, .. } => ask_asset_info.clone(),
        }
    }
}

impl fmt::Display for SwapOperation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SwapOperation::TerraSwap {
                offer_asset_info,
                ask_asset_info,
            } => write!(
                f,
                "TerraSwap {{ offer_asset_info: {}, ask_asset_info: {} }}",
                offer_asset_info, ask_asset_info
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

impl fmt::Display for SwapRoute {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "SwapRoute {{ offer_asset_info: {}, ask_asset_info: {}, swap_operations: {:?} }}",
            self.offer_asset_info, self.ask_asset_info, self.swap_operations
        )
    }
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    /// Execute multiple [SwapOperation]s, i.e. multi-hop swaps.
    ExecuteSwapOperations {
        operations: Vec<SwapOperation>,
        minimum_receive: Option<Uint128>,
        to: Option<String>,
    },
    /// Swap the offer to ask token. This message can only be called internally by the router contract.
    ExecuteSwapOperation {
        operation: SwapOperation,
        to: Option<String>,
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
}

#[cw_serde]
pub enum Cw20HookMsg {
    ExecuteSwapOperations {
        operations: Vec<SwapOperation>,
        minimum_receive: Option<Uint128>,
        to: Option<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Retrieves the configuration of the router.
    #[returns(ConfigResponse)]
    Config {},
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
    /// Gets the swap route for the given offer and ask assets.
    #[returns(Vec<SwapOperation>)]
    SwapRoute {
        offer_asset_info: AssetInfo,
        ask_asset_info: AssetInfo,
    },
}

// We define a custom struct for each query response
#[cw_serde]
pub struct ConfigResponse {
    pub terraswap_factory: String,
}

// We define a custom struct for each query response
#[cw_serde]
pub struct SimulateSwapOperationsResponse {
    pub amount: Uint128,
}

/// We currently take no arguments for migrations
#[cw_serde]
pub struct MigrateMsg {}
