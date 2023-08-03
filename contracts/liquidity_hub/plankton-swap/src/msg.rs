use cosmwasm_schema::{cw_serde, QueryResponses};
use white_whale::pool_network::{asset::PairType, pair::PoolFee};

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
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}
