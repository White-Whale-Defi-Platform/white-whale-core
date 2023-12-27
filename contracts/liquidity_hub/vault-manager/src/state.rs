use std::string::ToString;

use cosmwasm_std::{Deps, Order, StdResult, Storage, Uint128};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, Item, Map, MultiIndex, UniqueIndex};

use white_whale::pool_network::asset::AssetInfo;
use white_whale::vault_manager::{Config, Vault};

use crate::ContractError;

// A bool representing if a flashloan is being performed or not
pub const ONGOING_FLASHLOAN: Item<bool> = Item::new("ongoing_flashloan");

// Stores the balances of all assets in the contract before a flashloan, to compare after the
// messages are executed
pub const TEMP_BALANCES: Map<&[u8], Uint128> = Map::new("temp_balances");

// Contract's config
pub const CONFIG: Item<Config> = Item::new("config");
// Vault counter to keep track of the number of vaults. Used as identifier for vaults if no
// identifier is provided when creating a vault
pub const VAULT_COUNTER: Item<u64> = Item::new("vault_count");

pub const VAULTS: IndexedMap<String, Vault, VaultIndexes> = IndexedMap::new(
    "vaults",
    VaultIndexes {
        lp_asset: UniqueIndex::new(|v| v.lp_asset.to_string(), "vaults__lp_asset"),
        asset_info: MultiIndex::new(
            |_pk, v| v.asset.info.to_string(),
            "vaults",
            "vaults__asset_info",
        ),
    },
);

pub struct VaultIndexes<'a> {
    pub lp_asset: UniqueIndex<'a, String, Vault, String>,
    pub asset_info: MultiIndex<'a, String, Vault, String>,
}

impl<'a> IndexList<Vault> for VaultIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Vault>> + '_> {
        let v: Vec<&dyn Index<Vault>> = vec![&self.lp_asset, &self.asset_info];
        Box::new(v.into_iter())
    }
}

// settings for pagination
pub(crate) const MAX_LIMIT: u32 = 1_000;
const DEFAULT_LIMIT: u32 = 10;

/// Gets the vaults in the contract
pub fn get_vaults(
    storage: &dyn Storage,
    start_after: Option<Vec<u8>>,
    limit: Option<u32>,
) -> StdResult<Vec<Vault>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after).map(Bound::ExclusiveRaw);

    VAULTS
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, vault) = item?;

            Ok(vault)
        })
        .collect()
}

/// Calculates the item at which to start the range
fn calc_range_start(start_after: Option<Vec<u8>>) -> Option<Vec<u8>> {
    start_after.map(|item| {
        let mut v = item;
        v.push(1);
        v
    })
}

/// Gets vaults given an [AssetInfo]
pub fn get_vaults_by_asset_info(
    storage: &dyn Storage,
    asset_info: AssetInfo,
    start_after: Option<Vec<u8>>,
    limit: Option<u32>,
) -> StdResult<Vec<Vault>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after).map(Bound::ExclusiveRaw);

    VAULTS
        .idx
        .asset_info
        .prefix(asset_info.to_string())
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, vault) = item?;

            Ok(vault)
        })
        .collect()
}

/// Gets the vault given an lp asset as [AssetInfo]
pub fn get_vault_by_lp(deps: &Deps, lp_asset: &AssetInfo) -> Result<Vault, ContractError> {
    Ok(VAULTS
        .idx
        .lp_asset
        .item(deps.storage, lp_asset.to_string())?
        .map_or_else(|| Err(ContractError::NonExistentVault {}), Ok)?
        .1)
}

/// Gets the vault given its identifier
pub fn get_vault_by_identifier(
    deps: &Deps,
    vault_identifier: String,
) -> Result<Vault, ContractError> {
    VAULTS
        .may_load(deps.storage, vault_identifier)?
        .ok_or(ContractError::NonExistentVault {})
}
