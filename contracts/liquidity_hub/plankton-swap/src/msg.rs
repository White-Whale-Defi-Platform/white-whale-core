use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Decimal;
use white_whale::pool_network::{
    asset::{Asset, PairType},
    pair::PoolFee,
};

use crate::state::NAssets;

#[cw_serde]
pub struct InstantiateMsg {}

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
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}
