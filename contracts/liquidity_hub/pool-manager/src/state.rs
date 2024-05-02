use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Decimal, Deps};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, UniqueIndex};

pub use white_whale_std::pool_manager::Config;
use white_whale_std::pool_manager::{PairInfo, SwapOperation};

use crate::ContractError;

/// Holds information about the single side liquidity provision temporarily until the swap/liquidity
/// provision is completed
#[cw_serde]
pub struct SingleSideLiquidityProvisionBuffer {
    pub receiver: String,
    pub expected_offer_asset_balance_in_contract: Coin,
    pub expected_ask_asset_balance_in_contract: Coin,
    pub offer_asset_half: Coin,
    pub expected_ask_asset: Coin,
    pub liquidity_provision_data: LiquidityProvisionData,
}

/// Holds information about the intended liquidity provision when a user provides liquidity with a
/// single asset.
#[cw_serde]
pub struct LiquidityProvisionData {
    pub max_spread: Option<Decimal>,
    pub slippage_tolerance: Option<Decimal>,
    pub pair_identifier: String,
    pub unlocking_duration: Option<u64>,
    pub lock_position_identifier: Option<String>,
}

pub const TMP_SINGLE_SIDE_LIQUIDITY_PROVISION: Item<SingleSideLiquidityProvisionBuffer> =
    Item::new("tmp_single_side_liquidity_provision");

pub const PAIRS: IndexedMap<&str, PairInfo, PairIndexes> = IndexedMap::new(
    "pairs",
    PairIndexes {
        lp_asset: UniqueIndex::new(|v| v.lp_denom.to_string(), "pairs__lp_asset"),
    },
);

pub struct PairIndexes<'a> {
    pub lp_asset: UniqueIndex<'a, String, PairInfo, String>,
    // pub asset_info: MultiIndex<'a, String, NPairInfo, String>,
}

impl<'a> IndexList<PairInfo> for PairIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<PairInfo>> + '_> {
        let v: Vec<&dyn Index<PairInfo>> = vec![&self.lp_asset];
        Box::new(v.into_iter())
    }
}

/// Gets the pair given its identifier
pub fn get_pair_by_identifier(
    deps: &Deps,
    pair_identifier: &str,
) -> Result<PairInfo, ContractError> {
    PAIRS
        .may_load(deps.storage, pair_identifier)?
        .ok_or(ContractError::UnExistingPair {})
}

// Swap routes are used to establish defined routes for a given fee
// token to a desired fee token and is used for fee collection
#[cw_serde]
pub struct SwapOperations {
    // creator of the swap route, can remove it later
    pub creator: String,
    pub swap_operations: Vec<SwapOperation>,
}

pub const SWAP_ROUTES: Map<(&str, &str), SwapOperations> = Map::new("swap_routes");

pub const MANAGER_CONFIG: Item<Config> = Item::new("manager_config");
pub const PAIR_COUNTER: Item<u64> = Item::new("vault_count");
