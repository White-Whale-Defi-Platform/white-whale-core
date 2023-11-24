use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Deps, Order, QuerierWrapper, StdError, StdResult, Storage, Uint128};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, Item, Map, MultiIndex, UniqueIndex};
use white_whale::pool_network::asset::{Asset, AssetInfo, AssetInfoRaw, PairInfo, PairType};
use white_whale::pool_network::pair::{FeatureToggle, PoolFee};
use white_whale::pool_network::router::SwapOperation;

use crate::ContractError;
pub const LP_SYMBOL: &str = "uLP";
// Pairs are respresented as a Map of <&[u8], PairInfoRaw> where the key is the pair_key, which is a Vec<u8> of the two asset_infos sorted by their byte representation. This is done to ensure that the same pair is always represented by the same key, regardless of the order of the asset_infos.
// pub const PAIRS: Map<&[u8], NPairInfo> = Map::new("pair_info");
pub const PAIRS: IndexedMap<String, NPairInfo, PairIndexes> = IndexedMap::new(
    "vaults",
    PairIndexes {
        lp_asset: UniqueIndex::new(|v| v.liquidity_token.to_string(), "pairs__lp_asset"),
    },
);

pub struct PairIndexes<'a> {
    pub lp_asset: UniqueIndex<'a, String, NPairInfo, String>,
    // pub asset_info: MultiIndex<'a, String, Vault, String>,
}

impl<'a> IndexList<NPairInfo> for PairIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<NPairInfo>> + '_> {
        let v: Vec<&dyn Index<NPairInfo>> = vec![&self.lp_asset];
        Box::new(v.into_iter())
    }
}

/// Gets the pair given an lp asset as [AssetInfo]
pub fn get_pair_by_lp(deps: &Deps, lp_asset: &AssetInfo) -> Result<NPairInfo, ContractError> {
    Ok(PAIRS
        .idx
        .lp_asset
        .item(deps.storage, lp_asset.to_string())?
        .map_or_else(|| Err(ContractError::ExistingPair {}), Ok)?
        .1)
}

/// Gets the pair given its identifier
pub fn get_pair_by_identifier(
    deps: &Deps,
    vault_identifier: String,
) -> Result<NPairInfo, ContractError> {
    PAIRS
        .may_load(deps.storage, vault_identifier.clone())?
        .ok_or_else(|| ContractError::ExistingPair {})
}

// Used for PAIRS
pub fn pair_key(asset_infos: &[AssetInfoRaw]) -> Vec<u8> {
    let mut asset_infos = asset_infos.to_vec();
    asset_infos.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));

    asset_infos
        .iter()
        .flat_map(|info| info.as_bytes().to_vec())
        .collect()
}
pub fn get_decimals(pair_info: &NPairInfo) -> Vec<u8> {
    pair_info.asset_decimals.clone()
}

// Swap routes are used to establish defined routes for a given fee token to a desired fee token and is used for fee collection
pub const SWAP_ROUTES: Map<(&str, &str), Vec<SwapOperation>> = Map::new("swap_routes");

// Dyanmic Maps for Fee and Pair info
pub const COLLECTABLE_PROTOCOL_FEES: Map<&str, Vec<Asset>> = Map::new("collected_protocol_fees");
pub const TOTAL_COLLECTED_PROTOCOL_FEES: Map<&str, Vec<Asset>> =
    Map::new("total_collected_protocol_fees");
pub const ALL_TIME_BURNED_FEES: Map<&str, Vec<Asset>> = Map::new("all_time_burned_fees");

pub const MANAGER_CONFIG: Item<Config> = Item::new("manager_config");
pub const PAIR_COUNTER: Item<u64> = Item::new("vault_count");

// key : asset info / value: decimals
pub const ALLOW_NATIVE_TOKENS: Map<&[u8], u8> = Map::new("allow_native_token");
pub fn add_allow_native_token(
    storage: &mut dyn Storage,
    denom: String,
    decimals: u8,
) -> StdResult<()> {
    ALLOW_NATIVE_TOKENS.save(storage, denom.as_bytes(), &decimals)
}

// Define a structure for Fees which names a number of defined fee collection types, maybe leaving room for a custom room a user can use to pass a fee with a defined custom name
#[cw_serde]
pub enum Fee {
    Protocol,
    LiquidityProvider,
    FlashLoanFees,
    Custom(String),
}

// Store PairInfo to N
// We define a custom struct for which allows for dynamic but defined pairs
#[cw_serde]
pub struct NPairInfo {
    pub asset_infos: Vec<AssetInfo>,
    pub liquidity_token: AssetInfo,
    pub asset_decimals: Vec<u8>,
    pub balances: Vec<Uint128>,
    pub assets: Vec<Asset>,
    pub pair_type: PairType,
    pub pool_fees: PoolFee,
}
impl NPairInfo {}

// settings for pagination
const MAX_LIMIT: u32 = 1000;
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
    // The code ID for CW20
    pub token_code_id: u64,
    // We must set a creation fee on instantiation to prevent spamming of pools
    pub pool_creation_fee: Asset,
    //  Whether or not swaps, deposits, and withdrawals are enabled
    pub feature_toggle: FeatureToggle,
}

/// Stores the fee for an asset in the given fees_storage_item
pub fn store_fee(
    storage: &mut dyn Storage,
    fee_amount: Uint128,
    asset_id: String,
    fees_storage_item: Map<&str, Vec<Asset>>,
) -> StdResult<()> {
    let fees = fees_storage_item
        .load(storage, &asset_id)?
        .iter()
        .map(|fee_asset| {
            if fee_asset.clone().get_id() == asset_id {
                Asset {
                    info: fee_asset.info.clone(),
                    amount: fee_asset.amount + fee_amount,
                }
            } else {
                fee_asset.clone()
            }
        })
        .collect();

    fees_storage_item.save(storage, &asset_id, &fees)
}

/// Gets the fees for an asset from the given fees_storage_item
pub fn get_fees_for_asset(
    storage: &dyn Storage,
    asset_id: String,
    fees_storage_item: Map<&str, Vec<Asset>>,
) -> StdResult<Asset> {
    let fees = fees_storage_item
        .load(storage, &asset_id)?
        .iter()
        .find(|&fee_asset| fee_asset.clone().get_id() == asset_id)
        .cloned();

    if let Some(fees) = fees {
        Ok(fees)
    } else {
        Err(StdError::generic_err(format!(
            "Fees for asset {asset_id} not found"
        )))
    }
}
