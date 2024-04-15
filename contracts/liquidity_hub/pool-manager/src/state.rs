use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Deps};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, UniqueIndex};
use white_whale_std::pool_manager::{PairInfo, SwapOperation};
use white_whale_std::pool_network::pair::FeatureToggle;

use crate::ContractError;

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
// Remove after adding decimals to pair info
pub const NATIVE_TOKEN_DECIMALS: Map<&[u8], u8> = Map::new("allow_native_token");

// Swap routes are used to establish defined routes for a given fee token to a desired fee token and is used for fee collection
pub const SWAP_ROUTES: Map<(&str, &str), Vec<SwapOperation>> = Map::new("swap_routes");

pub const MANAGER_CONFIG: Item<Config> = Item::new("manager_config");
pub const PAIR_COUNTER: Item<u64> = Item::new("vault_count");

#[cw_serde]
pub struct Config {
    pub whale_lair_addr: Addr,
    pub owner: Addr,
    // We must set a creation fee on instantiation to prevent spamming of pools
    pub pool_creation_fee: Coin,
    //  Whether or not swaps, deposits, and withdrawals are enabled
    pub feature_toggle: FeatureToggle,
}
