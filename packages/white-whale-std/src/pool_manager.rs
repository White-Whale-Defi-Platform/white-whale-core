use std::fmt;

use crate::fee::PoolFee;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

/// The type of swap operation to perform.
#[cw_serde]
pub enum SwapOperation {
    /// A swap operation that uses the WhaleSwap router.
    WhaleSwap {
        /// The token denom to swap in.
        token_in_denom: String,
        /// The token denom returning from the swap.
        token_out_denom: String,
        /// The identifier of the pool to use for the swap.
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

/// The swap route structure
#[cw_serde]
pub struct SwapRoute {
    /// The offer asset denom, i.e. the asset that is being swapped.
    pub offer_asset_denom: String,
    /// The ask asset denom, i.e. the asset that is being received.
    pub ask_asset_denom: String,
    /// The swap operations to perform to get from offer asset to ask asset.
    pub swap_operations: Vec<SwapOperation>,
}

/// The response for the `SwapRoute` query.
#[cw_serde]
pub struct SwapRouteResponse {
    /// The swap route taken for the queried swap.
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

// Defines a structure for Fees which names a number of defined fee collection types, leaving
// room for a custom fee a user can use to pass a fee with a defined custom name
#[cw_serde]
pub enum FeeTypes {
    Protocol,
    LiquidityProvider,
    FlashLoanFees,
    Custom(String),
}

/// Params for the stable swap, used when changing the amplification factor
#[cw_serde]
pub struct StableSwapParams {
    /// Initial amplification factor
    pub initial_amp: String,
    /// Future amplification factor, i.e. target
    pub future_amp: String,
    /// Block height when the initial amplification factor kicks in. It goes gradually from the
    /// initial to the future amplification factor between the given blocks.
    pub initial_amp_block: String,
    /// Block height when the future amplification factor will be set.  It goes gradually from the
    /// initial to the future amplification factor between the given blocks.
    pub future_amp_block: String,
}

/// Contains the pool information
#[cw_serde]
pub struct PoolInfo {
    /// The asset denoms for the pool.
    pub asset_denoms: Vec<String>,
    /// The LP denom of the pool.
    pub lp_denom: String,
    /// The decimals for the given asset denoms, provided in the same order as asset_denoms.
    pub asset_decimals: Vec<u8>,
    /// The total amount of assets in the pool.
    pub assets: Vec<Coin>,
    /// The type of pool to create.
    pub pool_type: PoolType,
    /// The fees for the pool.
    pub pool_fees: PoolFee,
    //pub stable_swap_params: Option<StableSwapParams>
}

/// Possible pool types, it can be either a constant product (xyk) pool or a stable swap pool.
#[cw_serde]
pub enum PoolType {
    /// A stable swap pool.
    StableSwap {
        /// The amount of amplification to perform on the constant product part of the swap formula.
        amp: u64,
    },
    /// xyk pool
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

/// The contract configuration.
#[cw_serde]
pub struct Config {
    /// The address of the bonding manager contract.
    pub bonding_manager_addr: Addr,
    /// The address of the incentive manager contract.
    pub incentive_manager_addr: Addr,
    /// How much it costs to create a pool. It helps prevent spamming of new pools.
    pub pool_creation_fee: Coin,
    //  Whether or not swaps, deposits, and withdrawals are enabled
    pub feature_toggle: FeatureToggle,
}

#[cw_serde]
pub struct InstantiateMsg {
    /// The address of the bonding manager contract.
    pub bonding_manager_addr: String,
    /// The address of the incentive manager contract.
    pub incentive_manager_addr: String,
    /// How much it costs to create a pool. It helps prevent spamming of new pools.
    pub pool_creation_fee: Coin,
}

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
    Config,
    /// Retrieves the decimals for the given asset.
    #[returns(AssetDecimalsResponse)]
    AssetDecimals {
        /// The pool identifier to do the query for.
        pool_identifier: String,
        /// The queried denom in the given pool_identifier.
        denom: String,
    },
    /// Simulates a swap.
    #[returns(SimulationResponse)]
    Simulation {
        /// The offer asset to swap.
        offer_asset: Coin,
        /// The pool identifier to swap in.
        pool_identifier: String,
    },
    /// Simulates a reverse swap, i.e. given the ask asset, how much of the offer asset is needed
    /// to perform the swap.
    #[returns(ReverseSimulationResponse)]
    ReverseSimulation {
        /// The ask asset to get after the swap.
        ask_asset: Coin,
        /// The pool identifier to swap in.
        pool_identifier: String,
    },
    /// Gets the swap route for the given offer and ask assets.
    #[returns(SwapRouteResponse)]
    SwapRoute {
        /// The offer asset denom, i.e. the asset that is being swapped.
        offer_asset_denom: String,
        /// The ask asset denom, i.e. the asset that is being received.
        ask_asset_denom: String,
    },
    /// Gets all swap routes registered
    #[returns(SwapRoutesResponse)]
    SwapRoutes,
    /// Simulates swap operations.
    #[returns(SimulateSwapOperationsResponse)]
    SimulateSwapOperations {
        /// The amount to swap.
        offer_amount: Uint128,
        /// The operations to perform.
        operations: Vec<SwapOperation>,
    },
    /// Simulates a reverse swap operations, i.e. given the ask asset, how much of the offer asset
    /// is needed to perform the swap.
    #[returns(ReverseSimulateSwapOperationsResponse)]
    ReverseSimulateSwapOperations {
        /// The amount to get after the swap.
        ask_amount: Uint128,
        /// The operations to perform.
        operations: Vec<SwapOperation>,
    },
    /// Retrieves the pool information for the given pool identifier.
    #[returns(PoolInfoResponse)]
    Pool { pool_identifier: String },
    /// Retrieves the creator of the swap route to get from offer to ask asset. The creator of
    /// the swap route can remove it.
    #[returns(SwapRouteCreatorResponse)]
    SwapRouteCreator {
        /// The offer asset denom, i.e. the asset that is being swapped.
        offer_asset_denom: String,
        /// The ask asset denom, i.e. the asset that is being received.
        ask_asset_denom: String,
    },
}

/// The response for the `Config` query.
#[cw_serde]
pub struct ConfigResponse {
    /// The contract configuration.
    pub config: Config,
}

/// The response for the `SwapRoutes` query.
#[cw_serde]
pub struct SwapRoutesResponse {
    /// The swap routes registered in the contract.
    pub swap_routes: Vec<SwapRoute>,
}

/// The response for the `Pool` query.
#[cw_serde]
pub struct PoolInfoResponse {
    /// The pool information for the given pool identifier.
    pub pool_info: PoolInfo,
    /// The total LP tokens in the pool.
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
    /// The return amount of the ask asset given the offer amount.
    pub return_amount: Uint128,
    /// The spread amount of the swap.
    pub spread_amount: Uint128,
    /// The swap fee amount of the swap.
    pub swap_fee_amount: Uint128,
    /// The protocol fee amount of the swap.
    pub protocol_fee_amount: Uint128,
    /// The burn fee amount of the swap.
    pub burn_fee_amount: Uint128,
    /// The extra fees amount of the swap.
    pub extra_fees_amount: Uint128,
    /// The fee amount of the swap going to the osmosis community pool.
    #[cfg(feature = "osmosis")]
    pub osmosis_fee_amount: Uint128,
}

/// ReverseSimulationResponse returns reverse swap simulation response
#[cw_serde]
pub struct ReverseSimulationResponse {
    /// The amount of the offer asset needed to get the ask amount.
    pub offer_amount: Uint128,
    /// The spread amount of the swap.
    pub spread_amount: Uint128,
    /// The swap fee amount of the swap.
    pub swap_fee_amount: Uint128,
    /// The protocol fee amount of the swap.
    pub protocol_fee_amount: Uint128,
    /// The burn fee amount of the swap.
    pub burn_fee_amount: Uint128,
    /// The fee amount of the swap going to the osmosis community pool.
    #[cfg(feature = "osmosis")]
    pub osmosis_fee_amount: Uint128,
}

/// Pool feature toggle, can control whether swaps, deposits, and withdrawals are enabled.
#[cw_serde]
pub struct FeatureToggle {
    /// Whether or not swaps are enabled
    pub withdrawals_enabled: bool,
    /// Whether or not deposits are enabled
    pub deposits_enabled: bool,
    /// Whether or not swaps are enabled
    pub swaps_enabled: bool,
}

/// The response for the `SimulateSwapOperations` query.
#[cw_serde]
pub struct SimulateSwapOperationsResponse {
    /// The amount of the final token after the swap operations.
    pub amount: Uint128,
}

/// The response for the `ReverseSimulateSwapOperations` query.
#[cw_serde]
pub struct ReverseSimulateSwapOperationsResponse {
    /// The amount of the initial token needed to get the final token after the swap operations.
    pub amount: Uint128,
}

/// The response for the `SwapRouteCreator` query.
#[cw_serde]
pub struct SwapRouteCreatorResponse {
    /// The creator of the swap route.
    pub creator: String,
}
