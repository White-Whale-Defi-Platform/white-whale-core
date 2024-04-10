use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Coin, Deps, Order, StdResult, Storage};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, Item, Map, UniqueIndex};
use white_whale_std::pool_manager::{PairInfo, SwapOperation};
use white_whale_std::pool_network::asset::{Asset, AssetInfoRaw};
use white_whale_std::pool_network::pair::FeatureToggle;

use crate::ContractError;
pub const LP_SYMBOL: &str = "uLP";
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

pub fn get_decimals(pair_info: &PairInfo) -> Vec<u8> {
    pair_info.asset_decimals.clone()
}

// Swap routes are used to establish defined routes for a given fee token to a desired fee token and is used for fee collection
pub const SWAP_ROUTES: Map<(&str, &str), Vec<SwapOperation>> = Map::new("swap_routes");

//todo remove
// Dyanmic Maps for Fee and Pair info
pub const COLLECTABLE_PROTOCOL_FEES: Map<&str, Vec<Coin>> = Map::new("collected_protocol_fees");


//todo remove
pub const TOTAL_COLLECTED_PROTOCOL_FEES: Map<&str, Vec<Asset>> =
    Map::new("total_collected_protocol_fees");

//todo remove
pub const ALL_TIME_BURNED_FEES: Map<&str, Vec<Asset>> = Map::new("all_time_burned_fees");

pub const MANAGER_CONFIG: Item<Config> = Item::new("manager_config");
pub const PAIR_COUNTER: Item<u64> = Item::new("vault_count");

// settings for pagination
const MAX_LIMIT: u32 = 1000;
const DEFAULT_LIMIT: u32 = 10;

#[cw_serde]
pub struct Config {
    pub whale_lair_addr: Addr,
    pub owner: Addr,
    // We must set a creation fee on instantiation to prevent spamming of pools
    pub pool_creation_fee: Coin,
    //  Whether or not swaps, deposits, and withdrawals are enabled
    pub feature_toggle: FeatureToggle,
}
