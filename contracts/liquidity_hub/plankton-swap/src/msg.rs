use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Decimal;
use white_whale::pool_network::{
    asset::{Asset, AssetInfo, PairType},
    factory::NativeTokenDecimalsResponse,
    pair::PoolFee,
};

use crate::state::NAssets;

#[cw_serde]
pub struct InstantiateMsg {
    pub fee_collector_addr: String,
    pub token_code_id: u64,
    pub pair_code_id: u64,
    pub owner: String,
    pub pool_creation_fee: Vec<Asset>,
}

#[cw_serde]
pub enum ExecuteMsg {
    CreatePair {
        asset_infos: NAssets,
        // TODO: Remap to NPoolFee maybe
        pool_fees: PoolFee,
        pair_type: PairType,
        token_factory_lp: bool,
    },
    /// Provides liquidity to the pool
    ProvideLiquidity {
        assets: Vec<Asset>,
        slippage_tolerance: Option<Decimal>,
        receiver: Option<String>,
    },
    /// Swap an offer asset to the other
    Swap {
        offer_asset: Asset,
        ask_asset: AssetInfo,
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        to: Option<String>,
    },
    // /// Withdraws liquidity from the pool. Used only when the LP is a token factory token.
    WithdrawLiquidity {
        assets: Vec<Asset>,
    },
    /// Adds native token info to the contract so it can instantiate pair contracts that include it
    AddNativeTokenDecimals {
        denom: String,
        decimals: u8,
    },
}

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
}
