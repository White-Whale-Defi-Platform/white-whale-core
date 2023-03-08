use cosmwasm_schema::cw_serde;
use cosmwasm_std::{CanonicalAddr, DepsMut, Order, StdError, Uint128};
use cw_storage_plus::{Item, Map};
use pool_network::asset::{AssetInfo, AssetInfoRaw, PairType};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Migrate state of the factory from PascalCase to snake_case for the following items:
/// [`PairInfoRaw`], [`PairInfo`], [`AssetInfoRaw`], [`AssetRaw`], [`TmpPairInfo`]
/// as identified by commit c8d8462c6933b93245acdc8abbe303287fdc1951 which changed the structs to use
/// cw-serde's snake_case
pub fn migrate_to_v110(deps: DepsMut) -> Result<(), StdError> {
    // represent the old struct states
    // so we can deserialize from contract state
    // these are from commit 76f91fdb780677bcabfee631de6f9b973a36025f
    // it should be noted that this migration is not entirely accurate
    // as it depends on the only thing changing between the revisions being the casing format

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all(serialize = "snake_case"))]
    struct AssetRaw {
        pub info: AssetInfoRaw,
        pub amount: Uint128,
    }

    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all(serialize = "snake_case"))]
    pub enum AssetInfoRaw {
        Token { contract_addr: CanonicalAddr },
        NativeToken { denom: String },
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all(serialize = "snake_case"))]
    pub struct PairInfo {
        pub asset_infos: [AssetInfo; 2],
        pub contract_addr: String,
        pub liquidity_token: String,
        pub asset_decimals: [u8; 2],
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all(serialize = "snake_case"))]
    pub struct PairInfoRaw {
        pub asset_infos: [AssetInfoRaw; 2],
        pub contract_addr: CanonicalAddr,
        pub liquidity_token: CanonicalAddr,
        pub asset_decimals: [u8; 2],
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all(serialize = "snake_case"))]
    pub struct TmpPairInfo {
        pub pair_key: Vec<u8>,
        pub asset_infos: [AssetInfoRaw; 2],
        pub asset_decimals: [u8; 2],
    }

    const PAIRS: Map<&[u8], PairInfoRaw> = Map::new("pair_info");

    // force a serde deserialization into the old casing style, and then serialize into the new casing
    // back into the state
    let all_values = PAIRS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<Result<Vec<_>, _>>()?;

    all_values
        .into_iter()
        .try_for_each(|(key, value)| -> Result<(), StdError> {
            PAIRS.save(deps.storage, &key, &value)?;

            Ok(())
        })?;

    pub const TMP_PAIR_INFO: Item<TmpPairInfo> = Item::new("tmp_pair_info");
    let temp_pair_info = TMP_PAIR_INFO.may_load(deps.storage)?;
    if let Some(temp_pair_info) = temp_pair_info {
        TMP_PAIR_INFO.save(deps.storage, &temp_pair_info)?;
    }

    Ok(())
}

/// Migrate state of the factory for the StableSwap deployment
pub fn migrate_to_v120(deps: DepsMut) -> Result<(), StdError> {
    // migrate the TmpPairInfo struct changed in c9395c4f9d2a7b163056db0de33ab9066090969d
    // add the pair_type to it
    // we create a struct to represent the existing state, deserialize it, and serialize it to the new struct state
    // default pair_type to ConstantProduct
    // we also need to add this to PairInfo and PairInfoRaw

    #[cw_serde]
    pub struct OldTmpPairInfo {
        pub pair_key: Vec<u8>,
        pub asset_infos: [AssetInfoRaw; 2],
        pub asset_decimals: [u8; 2],
    }

    #[cw_serde]
    pub struct NewTmpPairInfo {
        pub pair_key: Vec<u8>,
        pub asset_infos: [AssetInfoRaw; 2],
        pub asset_decimals: [u8; 2],
        pub pair_type: PairType,
    }

    pub const OLD_TMP_PAIR_INFO: Item<OldTmpPairInfo> = Item::new("tmp_pair_info");
    pub const NEW_TMP_PAIR_INFO: Item<NewTmpPairInfo> = Item::new("tmp_pair_info");

    let temp_pair_info = OLD_TMP_PAIR_INFO.may_load(deps.storage)?;
    if let Some(temp_pair_info) = temp_pair_info {
        NEW_TMP_PAIR_INFO.save(
            deps.storage,
            &NewTmpPairInfo {
                asset_decimals: temp_pair_info.asset_decimals,
                asset_infos: temp_pair_info.asset_infos,
                pair_key: temp_pair_info.pair_key,
                pair_type: PairType::ConstantProduct,
            },
        )?;
    }

    // perform migration for PairInfo and PairInfoRaw
    #[cw_serde]
    pub struct PairInfo {
        pub asset_infos: [AssetInfo; 2],
        pub contract_addr: String,
        pub liquidity_token: String,
        pub asset_decimals: [u8; 2],
    }

    #[cw_serde]
    pub struct OldPairInfoRaw {
        pub asset_infos: [AssetInfoRaw; 2],
        pub contract_addr: CanonicalAddr,
        pub liquidity_token: CanonicalAddr,
        pub asset_decimals: [u8; 2],
    }
    #[cw_serde]
    pub struct NewPairInfoRaw {
        pub asset_infos: [AssetInfoRaw; 2],
        pub contract_addr: CanonicalAddr,
        pub liquidity_token: CanonicalAddr,
        pub asset_decimals: [u8; 2],
        pub pair_type: PairType,
    }

    const OLD_PAIRS: Map<&[u8], OldPairInfoRaw> = Map::new("pair_info");
    const NEW_PAIRS: Map<&[u8], NewPairInfoRaw> = Map::new("pair_info");

    let all_values = OLD_PAIRS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<Result<Vec<_>, _>>()?;

    all_values
        .into_iter()
        .try_for_each(|(key, pair)| -> Result<(), StdError> {
            NEW_PAIRS.save(
                deps.storage,
                &key,
                &NewPairInfoRaw {
                    asset_infos: pair.asset_infos,
                    contract_addr: pair.contract_addr,
                    liquidity_token: pair.liquidity_token,
                    asset_decimals: pair.asset_decimals,
                    pair_type: PairType::ConstantProduct,
                },
            )?;

            Ok(())
        })?;

    Ok(())
}
