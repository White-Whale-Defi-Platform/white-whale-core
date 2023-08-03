use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Order, StdResult, Storage};
use cw_storage_plus::{Bound, Item, Map};
use white_whale::pool_network::asset::{Asset, AssetInfo, AssetInfoRaw, PairInfo, PairType};
use white_whale::pool_network::pair::FeatureToggle;
use white_whale::pool_network::router::SwapOperation;

// Pairs are respresented as a Map of <&[u8], PairInfoRaw> where the key is the pair_key, which is a Vec<u8> of the two asset_infos sorted by their byte representation. This is done to ensure that the same pair is always represented by the same key, regardless of the order of the asset_infos.
pub const PAIRS: Map<&[u8], NPairInfo> = Map::new("pair_info");
// Used for PAIRS
pub fn pair_key(asset_infos: &[AssetInfoRaw]) -> Vec<u8> {
    let mut asset_infos = asset_infos.to_vec();
    asset_infos.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));

    asset_infos
        .iter()
        .flat_map(|info| info.as_bytes().to_vec())
        .collect()
}
// Swap routes are used to establish defined routes for a given fee token to a desired fee token and is used for fee collection
pub const SWAP_ROUTES: Map<(&str, &str), Vec<SwapOperation>> = Map::new("swap_routes");

// Dyanmic Maps for Fee and Pair info
pub const PAIR_INFO: Map<&str, Item<PairInfo>> = Map::new("pair_info");
pub const COLLECTABLE_PROTOCOL_FEES: Map<&str, Vec<Asset>> = Map::new("collected_protocol_fees");
pub const TOTAL_COLLECTED_PROTOCOL_FEES: Map<&str, Vec<Asset>> =
    Map::new("total_collected_protocol_fees");
pub const MANAGER_CONFIG: Item<Config> = Item::new("manager_config");

// Define a structure for Fees which names a number of defined fee collection types, maybe leaving room for a custom room a user can use to pass a fee with a defined custom name
#[cw_serde]
pub enum Fee {
    Protocol,
    LiquidityProvider,
    FlashLoanFees,
    Custom(String),
}

#[cw_serde]
pub enum NAssets {
    TWO([AssetInfo; 2]),
    THREE([AssetInfo; 3]),
    // N Assets is also possible where N is the number of assets in the pool
    // Note Vec with an unbounded size, we need to have extra parsing on this one to eventually store [AssetInfoRaw; N]
    N(Vec<AssetInfo>),
}

#[cw_serde]
pub enum NDecimals {
    TWO([u8; 2]),
    THREE([u8; 3]),
    N(Vec<u8>),
}

// Use above enums to enable a somewhat dynamic PairInfo which can support a normal 2 asset or a 3 pair. The design can be expanded to N types
#[cw_serde]
pub struct TmpPairInfo {
    pub pair_key: Vec<u8>,
    pub asset_infos: NAssets,
    pub asset_decimals: NDecimals,
    pub pair_type: PairType,
}
pub const TMP_PAIR_INFO: Item<TmpPairInfo> = Item::new("tmp_pair_info");
// Store PairInfo to N
// We define a custom struct for which allows for dynamic but defined pairs
#[cw_serde]
pub struct NPairInfo {
    pub asset_infos: NAssets,
    pub liquidity_token: AssetInfo,
    pub asset_decimals: NDecimals,
    pub pair_type: PairType,
}

// // We could store trios separate to pairs but if we use trio key properly theres no need really
// pub const TRIOS: Map<&[u8], TrioInfoRaw> = Map::new("trio_info");
/// Used for TRIOS or to just store a trio in PAIRS, takes a vec of 3 asset infos and returns a Vec<u8> of the asset infos sorted by their byte representation
/// The trio key can be used to ensure no clashes with any of the other 2 pair pools
pub fn trio_key(asset_infos: &[AssetInfoRaw; 3]) -> Vec<u8> {
    let mut asset_infos = asset_infos.to_vec();
    asset_infos.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));

    [
        asset_infos[0].as_bytes(),
        asset_infos[1].as_bytes(),
        asset_infos[2].as_bytes(),
    ]
    .concat()
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
pub fn read_pairs(
    storage: &dyn Storage,
    _api: &dyn Api,
    start_after: Option<[AssetInfoRaw; 2]>,
    limit: Option<u32>,
) -> StdResult<Vec<NPairInfo>> {
    // Note PairInfo may need to be refactored to handle the 2or3 design
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after).map(Bound::ExclusiveRaw);

    PAIRS
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, v) = item?;
            Ok(v)
        })
        .collect::<StdResult<Vec<NPairInfo>>>()
}

// this will set the first key after the provided key, by appending a 1 byte
fn calc_range_start(start_after: Option<[AssetInfoRaw; 2]>) -> Option<Vec<u8>> {
    start_after.map(|asset_infos| {
        let mut asset_infos = asset_infos.to_vec();
        asset_infos.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));

        let mut v = [asset_infos[0].as_bytes(), asset_infos[1].as_bytes()]
            .concat()
            .as_slice()
            .to_vec();
        v.push(1);
        v
    })
}

#[cw_serde]
pub struct Config {
    pub fee_collector_addr: Addr,
    pub owner: Addr,
    // The code ID for the pair and tokens
    pub pair_code_id: u64,
    pub token_code_id: u64,
    // We must set a creation fee on instantiation to prevent spamming of pools
    pub pool_creation_fee: Vec<Asset>,
    //  Whether or not swaps, deposits, and withdrawals are enabled
    pub feature_toggle: FeatureToggle,
}
