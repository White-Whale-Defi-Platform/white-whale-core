use std::fmt;

use crate::fee::PoolFee;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

#[cw_serde]
pub enum SwapOperation {
    WhaleSwap {
        token_in_denom: String,
        token_out_denom: String,
        pool_identifier: String,
    },
}

impl SwapOperation {
    /// Retrieves the `token_in_denom` used for this swap operation.
    pub fn get_input_asset_info(&self) -> &String {
        match self {
            SwapOperation::WhaleSwap { token_in_denom, .. } => token_in_denom,
        }
    }

    pub fn get_target_asset_info(&self) -> String {
        match self {
            SwapOperation::WhaleSwap {
                token_out_denom, ..
            } => token_out_denom.clone(),
        }
    }
}

impl fmt::Display for SwapOperation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SwapOperation::WhaleSwap {
                token_in_denom,
                token_out_denom,
                pool_identifier,
            } => write!(
                f,
                "WhaleSwap {{ token_in_info: {token_in_denom}, token_out_info: {token_out_denom}, pool_identifier: {pool_identifier} }}"
            ),

        }
    }
}

#[cw_serde]
pub struct SwapRoute {
    pub offer_asset_denom: String,
    pub ask_asset_denom: String,
    pub swap_operations: Vec<SwapOperation>,
}

// Used for all swap routes
#[cw_serde]
pub struct SwapRouteResponse {
    pub swap_route: SwapRoute,
}

impl fmt::Display for SwapRoute {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "SwapRoute {{ offer_asset_info: {}, ask_asset_info: {}, swap_operations: {:?} }}",
            self.offer_asset_denom, self.ask_asset_denom, self.swap_operations
        )
    }
}

// Define a structure for Fees which names a number of defined fee collection types, maybe leaving room for a custom room a user can use to pass a fee with a defined custom name
#[cw_serde]
pub enum FeeTypes {
    Protocol,
    LiquidityProvider,
    FlashLoanFees,
    Custom(String),
}

#[cw_serde]

pub struct StableSwapParams {
    pub initial_amp: String,
    pub future_amp: String,
    pub initial_amp_block: String,
    pub future_amp_block: String,
}

// Store PoolInfo to N
// We define a custom struct for which allows for dynamic but defined pools
#[cw_serde]
pub struct PoolInfo {
    pub asset_denoms: Vec<String>,
    pub lp_denom: String,
    pub asset_decimals: Vec<u8>,
    pub assets: Vec<Coin>,
    pub pool_type: PoolType,
    pub pool_fees: PoolFee,
    //pub stable_swap_params: Option<StableSwapParams>
}
impl PoolInfo {}

#[cw_serde]
pub enum PoolType {
    StableSwap {
        /// The amount of amplification to perform on the constant product part of the swap formula.
        amp: u64,
    },
    ConstantProduct,
}

impl PoolType {
    /// Gets a string representation of the pair type
    pub fn get_label(&self) -> &str {
        match self {
            PoolType::ConstantProduct => "ConstantProduct",
            PoolType::StableSwap { .. } => "StableSwap",
        }
    }
}

#[cw_serde]
pub struct Config {
    /// The address of the bonding manager contract.
    pub bonding_manager_addr: Addr,
    /// The address of the incentive manager contract.
    pub incentive_manager_addr: Addr,
    // We must set a creation fee on instantiation to prevent spamming of pools
    pub pool_creation_fee: Coin,
    //  Whether or not swaps, deposits, and withdrawals are enabled
    pub feature_toggle: FeatureToggle,
}

#[cw_serde]
pub struct InstantiateMsg {
    pub bonding_manager_addr: String,
    pub incentive_manager_addr: String,
    pub pool_creation_fee: Coin,
}

/// The migrate message
#[cw_serde]
pub struct MigrateMsg {}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    /// Creates a new pool.
    CreatePool {
        /// The asset denoms for the pool.
        asset_denoms: Vec<String>,
        /// The decimals for the given asset denoms, provided in the same order as `asset_denoms`.
        asset_decimals: Vec<u8>,
        /// The fees for the pool.
        pool_fees: PoolFee,
        /// The type of pool to create.
        pool_type: PoolType,
        /// The identifier for the pool.
        pool_identifier: Option<String>,
    },
    /// Provides liquidity to the pool
    ProvideLiquidity {
        /// A percentage value representing the acceptable slippage for the operation.
        /// When provided, if the slippage exceeds this value, the liquidity provision will not be
        /// executed.
        slippage_tolerance: Option<Decimal>,
        /// The maximum allowable spread between the bid and ask prices for the pool.
        /// When provided, if the spread exceeds this value, the liquidity provision will not be
        /// executed.
        max_spread: Option<Decimal>,
        /// The receiver of the LP
        receiver: Option<String>,
        /// The identifier for the pool to provide liquidity for.
        pool_identifier: String,
        /// The amount of time in seconds to unlock tokens if taking part on the incentives. If not passed,
        /// the tokens will not be locked and the LP tokens will be returned to the user.
        unlocking_duration: Option<u64>,
        /// The identifier of the position to lock the LP tokens in the incentive manager, if any.
        lock_position_identifier: Option<String>,
    },
    /// Swap an offer asset to the other
    Swap {
        /// The return asset of the swap.
        ask_asset_denom: String,
        /// The belief price of the swap.
        belief_price: Option<Decimal>,
        /// The maximum spread to incur when performing the swap. If the spread exceeds this value,
        /// the swap will not be executed.
        max_spread: Option<Decimal>,
        /// The recipient of the output tokens. If not provided, the tokens will be sent to the sender
        /// of the message.
        receiver: Option<String>,
        /// The identifier for the pool to swap in.
        pool_identifier: String,
    },
    /// Withdraws liquidity from the pool.
    WithdrawLiquidity { pool_identifier: String },
    /// Execute multiple [`SwapOperations`] to allow for multi-hop swaps.
    ExecuteSwapOperations {
        /// The operations that should be performed in sequence.
        ///
        /// The amount in each swap will be the output from the previous swap.
        ///
        /// The first swap will use whatever funds are sent in the [`MessageInfo`].
        operations: Vec<SwapOperation>,
        /// The minimum amount of the output (i.e., final swap operation token) required for the message to succeed.
        minimum_receive: Option<Uint128>,
        /// The (optional) recipient of the output tokens.
        ///
        /// If left unspecified, tokens will be sent to the sender of the message.
        receiver: Option<String>,
        /// The (optional) maximum spread to incur when performing any swap.
        ///
        /// If left unspecified, there is no limit to what spread the transaction can incur.
        max_spread: Option<Decimal>,
    },
    /// Adds swap routes to the router.
    AddSwapRoutes { swap_routes: Vec<SwapRoute> },
    /// Removes swap routes from the router.
    RemoveSwapRoutes { swap_routes: Vec<SwapRoute> },
    /// Updates the configuration of the contract.
    /// If a field is not specified (i.e., set to `None`), it will not be modified.
    UpdateConfig {
        /// The new whale-lair contract address.
        whale_lair_addr: Option<String>,
        /// The new fee that must be paid when a pool is created.
        pool_creation_fee: Option<Coin>,
        /// The new feature toggles of the contract, allowing fine-tuned
        /// control over which operations are allowed.
        feature_toggle: Option<FeatureToggle>,
    },
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Retrieves the contract's config.
    #[returns(ConfigResponse)]
    Config {},

    /// Retrieves the decimals for the given asset.
    #[returns(AssetDecimalsResponse)]
    AssetDecimals {
        pool_identifier: String,
        denom: String,
    },

    /// Simulates a swap.
    #[returns(SimulationResponse)]
    Simulation {
        offer_asset: Coin,
        pool_identifier: String,
    },
    /// Simulates a reverse swap, i.e. given the ask asset, how much of the offer asset is needed to
    /// perform the swap.
    #[returns(ReverseSimulationResponse)]
    ReverseSimulation {
        ask_asset: Coin,
        offer_asset: Coin,
        pool_identifier: String,
    },

    /// Gets the swap route for the given offer and ask assets.
    #[returns(SwapRouteResponse)]
    SwapRoute {
        offer_asset_denom: String,
        ask_asset_denom: String,
    },
    /// Gets all swap routes registered
    #[returns(SwapRoutesResponse)]
    SwapRoutes {},

    /// Simulates swap operations.
    #[returns(SimulateSwapOperationsResponse)]
    SimulateSwapOperations {
        offer_amount: Uint128,
        operations: Vec<SwapOperation>,
    },
    /// Simulates a reverse swap operations, i.e. given the ask asset, how much of the offer asset
    /// is needed to perform the swap.
    #[returns(ReverseSimulateSwapOperationsResponse)]
    ReverseSimulateSwapOperations {
        ask_amount: Uint128,
        operations: Vec<SwapOperation>,
    },

    #[returns(PoolInfoResponse)]
    Pool { pool_identifier: String },
    /// Retrieves the creator of the swap routes that can then remove them.
    #[returns(SwapRouteCreatorResponse)]
    SwapRouteCreator {
        offer_asset_denom: String,
        ask_asset_denom: String,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    pub config: Config,
}

#[cw_serde]
pub struct SwapRoutesResponse {
    pub swap_routes: Vec<SwapRoute>,
}

#[cw_serde]
pub struct PoolInfoResponse {
    pub pool_info: PoolInfo,
    pub total_share: Coin,
}

/// The response for the `AssetDecimals` query.
#[cw_serde]
pub struct AssetDecimalsResponse {
    /// The pool identifier to do the query for.
    pub pool_identifier: String,
    /// The queried denom in the given pool_identifier.
    pub denom: String,
    /// The decimals for the requested denom.
    pub decimals: u8,
}

/// SimulationResponse returns swap simulation response
#[cw_serde]
pub struct SimulationResponse {
    pub return_amount: Uint128,
    pub spread_amount: Uint128,
    pub swap_fee_amount: Uint128,
    pub protocol_fee_amount: Uint128,
    pub burn_fee_amount: Uint128,
    #[cfg(feature = "osmosis")]
    pub osmosis_fee_amount: Uint128,
}

/// ReverseSimulationResponse returns reverse swap simulation response
#[cw_serde]
pub struct ReverseSimulationResponse {
    pub offer_amount: Uint128,
    pub spread_amount: Uint128,
    pub swap_fee_amount: Uint128,
    pub protocol_fee_amount: Uint128,
    pub burn_fee_amount: Uint128,
    #[cfg(feature = "osmosis")]
    pub osmosis_fee_amount: Uint128,
}

/// Pool feature toggle
#[cw_serde]
pub struct FeatureToggle {
    pub withdrawals_enabled: bool,
    pub deposits_enabled: bool,
    pub swaps_enabled: bool,
}

// We define a custom struct for each query response
#[cw_serde]
pub struct SimulateSwapOperationsResponse {
    pub amount: Uint128,
}

#[cw_serde]
pub struct ReverseSimulateSwapOperationsResponse {
    pub amount: Uint128,
}

#[cw_serde]
pub struct SwapRouteCreatorResponse {
    pub creator: String,
}
