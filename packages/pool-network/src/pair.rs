use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, StdError, StdResult, Uint128};
use cw20::Cw20ReceiveMsg;

use white_whale::fee::Fee;

use crate::asset::{Asset, AssetInfo, PairInfo, PairType};

#[cw_serde]
pub struct InstantiateMsg {
    /// Asset infos
    pub asset_infos: [AssetInfo; 2],
    /// Token contract code id for initialization
    pub token_code_id: u64,
    pub asset_decimals: [u8; 2],
    pub pool_fees: PoolFee,
    pub fee_collector_addr: String,
    /// The type of pair to create
    pub pair_type: PairType,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Used to trigger the [Cw20HookMsg] messages
    Receive(Cw20ReceiveMsg),
    /// Provides liquidity to the pool
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
    /// Collects the Protocol fees accrued by the pool
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
    /// Withdraws liquidity
    WithdrawLiquidity {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Retrieves the info for the pair.
    #[returns(PairInfo)]
    Pair {},
    /// Retrieves the configuration of the pool.
    #[returns(ConfigResponse)]
    Config {},
    /// Retrieves the protocol fees that have been accrued. If `all_time` is `true`, it will return
    /// the fees collected since the inception of the pool. On the other hand, if `all_time` is set
    /// to `false`, only the fees that has been accrued by the pool but not collected by the fee
    /// collector will be returned.
    #[returns(ProtocolFeesResponse)]
    ProtocolFees {
        asset_id: Option<String>,
        all_time: Option<bool>,
    },
    /// Retrieves the fees that have been burned by the pool.
    #[returns(ProtocolFeesResponse)]
    BurnedFees { asset_id: Option<String> },
    /// Retrieves the pool information.
    #[returns(PoolResponse)]
    Pool {},
    /// Simulates a swap.
    #[returns(SimulationResponse)]
    Simulation { offer_asset: Asset },
    /// Simulates a reverse swap, i.e. given the ask asset, how much of the offer asset is needed to
    /// perform the swap.
    #[returns(ReverseSimulationResponse)]
    ReverseSimulation { ask_asset: Asset },
}

/// Pool feature toggle
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
    pub burn_fee: Fee,
}

impl PoolFee {
    /// Checks that the given [PoolFee] is valid, i.e. the fees provided are valid, and they don't
    /// exceed 100% together
    pub fn is_valid(&self) -> StdResult<()> {
        self.protocol_fee.is_valid()?;
        self.swap_fee.is_valid()?;
        self.burn_fee.is_valid()?;

        if self
            .protocol_fee
            .share
            .checked_add(self.swap_fee.share)?
            .checked_add(self.burn_fee.share)?
            >= Decimal::percent(100)
        {
            return Err(StdError::generic_err("Invalid fees"));
        }
        Ok(())
    }
}

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub fee_collector_addr: Addr,
    pub pool_fees: PoolFee,
    pub feature_toggle: FeatureToggle,
}

pub type ConfigResponse = Config;

/// We define a custom struct for each query response
#[cw_serde]
pub struct PoolResponse {
    pub assets: Vec<Asset>,
    pub total_share: Uint128,
}

/// SimulationResponse returns swap simulation response
#[cw_serde]
pub struct SimulationResponse {
    pub return_amount: Uint128,
    pub spread_amount: Uint128,
    pub swap_fee_amount: Uint128,
    pub protocol_fee_amount: Uint128,
    pub burn_fee_amount: Uint128,
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
    pub burn_fee_amount: Uint128,
}

/// We currently take no arguments for migrations
#[cw_serde]
pub struct MigrateMsg {}
