use std::fmt;

use crate::{fee::Fee, pool_network::{
    asset::PairType,
    factory::NativeTokenDecimalsResponse,
    pair::{ReverseSimulationResponse, SimulationResponse},
}};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Decimal, StdError, StdResult, Uint128, Uint256};
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
    pub offer_asset_denom: String,
    pub ask_asset_denom: String,
    pub swap_route: Vec<SwapOperation>,
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

/// Represents the fee structure for transactions within a pool.
/// 
/// 
/// # Fields
/// - `protocol_fee`: The fee percentage charged by the protocol on each transaction to support
///   operational and developmental needs.
/// - `swap_fee`: The fee percentage allocated to liquidity providers as a reward for supplying
///   liquidity to the pool, incentivizing participation and ensuring pool health.
/// - `burn_fee`: A fee percentage that is burned on each transaction, helping manage the token
///   economy by reducing supply over time, potentially increasing token value.
/// - `osmosis_fee` (optional): Specific to the Osmosis feature, this fee is charged on each
///   transaction when the Osmosis feature is enabled, supporting specific ecosystem requirements.
/// - `extra_fees`: A vector of custom fees allowing for extensible and adaptable fee structures
///   to meet diverse and evolving needs. Validation ensures that the total of all fees does not
///   exceed 100%, maintaining fairness and avoiding overcharging.
///
/// # Features
/// - `osmosis`: Enables the `osmosis_fee` field, integrating specific fee requirements for the
///   Osmosis protocol within the pool's fee structure.
#[cw_serde]
pub struct PoolFee {
    /// Fee percentage charged on each transaction for the protocol's benefit.
    pub protocol_fee: Fee,
    
    /// Fee percentage allocated to liquidity providers on each swap. 
    pub swap_fee: Fee,
    
    /// Fee percentage that is burned on each transaction. Burning a portion of the transaction fee
    /// helps in reducing the overall token supply.
    pub burn_fee: Fee,
    
    /// Fee percentage charged on each transaction specifically for Osmosis integrations. This fee
    /// is only applicable when the `osmosis` feature is enabled
    #[cfg(feature = "osmosis")]
    pub osmosis_fee: Fee,
    
    /// A list of custom, additional fees that can be defined for specific use cases or additional
    /// functionalities. This vector enables the flexibility to introduce new fees without altering
    /// the core fee structure. Total of all fees, including custom ones, is validated to not exceed
    /// 100%, ensuring a balanced and fair fee distribution.
    pub extra_fees: Vec<Fee>,
}
impl PoolFee {
    /// Validates the PoolFee structure to ensure no individual fee is zero or negative
    /// and the sum of all fees does not exceed 20%.
    pub fn is_valid(&self) -> StdResult<()> {
        let mut total_share = Decimal::zero();

        // Validate predefined fees and accumulate their shares
        let predefined_fees = [
            &self.protocol_fee,
            &self.swap_fee,
            &self.burn_fee,
            #[cfg(feature = "osmosis")]
            &self.osmosis_fee,
        ];

        for fee in predefined_fees.iter().filter_map(|f| Some(*f)) {
            fee.is_valid()?; // Validates the fee is not >= 100%
            total_share = total_share + fee.share;
        }

        // Validate extra fees and accumulate their shares
        for fee in &self.extra_fees {
            fee.is_valid()?; // Validates the fee is not >= 100%
            total_share = total_share + fee.share;
        }

        // Check if the total share exceeds 20%
        if total_share > Decimal::percent(20) {
            return Err(StdError::generic_err("Total fees cannot exceed 20%"));
        }

        Ok(())
    }

    /// Computes and applies all defined fees to a given amount.
    /// Returns the total amount of fees deducted.
    pub fn compute_and_apply_fees(&self, amount: Uint256) -> StdResult<Uint128> {
        let mut total_fee_amount = Uint256::zero();

        // Compute protocol fee
        let protocol_fee_amount = self.protocol_fee.compute(amount);
        total_fee_amount += protocol_fee_amount;

        // Compute swap fee
        let swap_fee_amount = self.swap_fee.compute(amount);
        total_fee_amount += swap_fee_amount;

        // Compute burn fee
        let burn_fee_amount = self.burn_fee.compute(amount);
        total_fee_amount += burn_fee_amount;

        // Compute osmosis fee if applicable
        #[cfg(feature = "osmosis")]{
        let osmosis_fee_amount = self.osmosis_fee.compute(amount);

        total_fee_amount += osmosis_fee_amount;
        }

        // Compute extra fees
        for extra_fee in &self.extra_fees {
            let extra_fee_amount = extra_fee.compute(amount);
            total_fee_amount += extra_fee_amount;
        }

        // Convert the total fee amount to Uint128 (or handle potential conversion failure)
        Uint128::try_from(total_fee_amount).map_err(|_| StdError::generic_err("Fee conversion error"))
    }
}

#[cw_serde]

pub struct StableSwapParams {
    pub initial_amp: String,
    pub future_amp: String,
    pub initial_amp_block: String,
    pub future_amp_block: String,
}

// Store PairInfo to N
// We define a custom struct for which allows for dynamic but defined pairs
#[cw_serde]
pub struct PairInfo {
    pub asset_denoms: Vec<String>,
    pub lp_denom: String,
    pub asset_decimals: Vec<u8>,
    pub assets: Vec<Coin>,
    pub pair_type: PairType,
    pub pool_fees: PoolFee,
    // TODO: Add stable swap params
    // pub stable_swap_params: Option<StableSwapParams>
}
impl PairInfo {}

#[cw_serde]
pub struct InstantiateMsg {
    pub fee_collector_addr: String,
    pub owner: String,
    pub pool_creation_fee: Coin,
}

/// The migrate message
#[cw_serde]
pub struct MigrateMsg {}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    CreatePair {
        asset_denoms: Vec<String>,
        // TODO: Remap to NPoolFee maybe
        pool_fees: PoolFee,
        pair_type: PairType,
        pair_identifier: Option<String>,
    },
    /// Provides liquidity to the pool
    ProvideLiquidity {
        slippage_tolerance: Option<Decimal>,
        receiver: Option<String>,
        pair_identifier: String,
    },
    /// Swap an offer asset to the other
    Swap {
        offer_asset: Coin,
        ask_asset_denom: String,
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        to: Option<String>,
        pair_identifier: String,
    },
    // /// Withdraws liquidity from the pool.
    WithdrawLiquidity {
        pair_identifier: String,
    },
    /// Adds native token info to the contract so it can instantiate pair contracts that include it
    AddNativeTokenDecimals {
        denom: String,
        decimals: u8,
    },

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
        to: Option<String>,
        /// The (optional) maximum spread to incur when performing any swap.
        ///
        /// If left unspecified, there is no limit to what spread the transaction can incur.
        max_spread: Option<Decimal>,
    },
    // /// Swap the offer to ask token. This message can only be called internally by the router contract.
    // ExecuteSwapOperation {
    //     operation: SwapOperation,
    //     to: Option<String>,
    //     max_spread: Option<Decimal>,
    // },
    // /// Checks if the swap amount exceeds the minimum_receive. This message can only be called
    // /// internally by the router contract.
    // AssertMinimumReceive {
    //     asset_info: AssetInfo,
    //     prev_balance: Uint128,
    //     minimum_receive: Uint128,
    //     receiver: String,
    // },
    /// Adds swap routes to the router.
    AddSwapRoutes {
        swap_routes: Vec<SwapRoute>,
    },
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
        offer_asset: Coin,
        ask_asset: Coin,
        pair_identifier: String,
    },
    /// Simulates a reverse swap, i.e. given the ask asset, how much of the offer asset is needed to
    /// perform the swap.
    #[returns(ReverseSimulationResponse)]
    ReverseSimulation {
        ask_asset: Coin,
        offer_asset: Coin,
        pair_identifier: String,
    },

    /// Gets the swap route for the given offer and ask assets.
    #[returns(Vec<SwapOperation>)]
    SwapRoute {
        offer_asset_denom: String,
        ask_asset_denom: String,
    },
    /// Gets all swap routes registered
    #[returns(Vec<SwapRouteResponse>)]
    SwapRoutes {},

    // /// Simulates swap operations.
    // #[returns(SimulateSwapOperationsResponse)]
    // SimulateSwapOperations {
    //     offer_amount: Uint128,
    //     operations: Vec<SwapOperation>,
    // },
    // /// Simulates a reverse swap operations, i.e. given the ask asset, how much of the offer asset
    // /// is needed to perform the swap.
    // #[returns(SimulateSwapOperationsResponse)]
    // ReverseSimulateSwapOperations {
    //     ask_amount: Uint128,
    //     operations: Vec<SwapOperation>,
    // },
    #[returns(PairInfo)]
    Pair { pair_identifier: String },
}
