#![cfg(not(tarpaulin_include))]
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, CanonicalAddr, Decimal, DepsMut, StdError, Uint128};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::helpers::instantiate_fees;
use pool_network::asset::{AssetInfo, AssetInfoRaw, PairType};
use pool_network::pair::{Config, FeatureToggle};
use white_whale::fee::Fee;

use crate::state::{ALL_TIME_BURNED_FEES, CONFIG, PAIR_INFO};

/// Migrate state of the factory from PascalCase to snake_case for the following items:
/// [`PairInfoRaw`], [`PairInfo`]
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

    pub const PAIR_INFO: Item<PairInfoRaw> = Item::new("pair_info");

    // force a serde deserialization into the old casing style, and then serialize into the new casing
    // back into the state
    let pair_info = PAIR_INFO.load(deps.storage)?;
    PAIR_INFO.save(deps.storage, &pair_info)?;

    Ok(())
}

pub fn migrate_to_v120(deps: DepsMut) -> Result<(), StdError> {
    #[cw_serde]
    struct ConfigV110 {
        pub owner: Addr,
        pub fee_collector_addr: Addr,
        pub pool_fees: PoolFeeV110,
        pub feature_toggle: FeatureToggle,
    }

    #[cw_serde]
    struct PoolFeeV110 {
        pub protocol_fee: Fee,
        pub swap_fee: Fee,
    }

    const CONFIG_V110: Item<ConfigV110> = Item::new("config");
    let config_v110 = CONFIG_V110.load(deps.storage)?;

    // Add burn fee to config. Zero fee is used as default.
    let config = Config {
        owner: config_v110.owner,
        fee_collector_addr: config_v110.fee_collector_addr,
        pool_fees: pool_network::pair::PoolFee {
            protocol_fee: config_v110.pool_fees.protocol_fee,
            swap_fee: config_v110.pool_fees.swap_fee,
            burn_fee: Fee {
                share: Decimal::zero(),
            },
        },
        feature_toggle: config_v110.feature_toggle,
    };

    CONFIG.save(deps.storage, &config)?;

    // Instantiates the ALL_TIME_BURNED_FEES
    let pair_info = PAIR_INFO.load(deps.storage)?;
    let asset_info_0 = pair_info.asset_infos[0].to_normal(deps.api)?;
    let asset_info_1 = pair_info.asset_infos[1].to_normal(deps.api)?;

    instantiate_fees(
        deps.storage,
        asset_info_0,
        asset_info_1,
        ALL_TIME_BURNED_FEES,
    )?;

    Ok(())
}

/// Migrate to the StableSwap deployment
///
/// Default to a ConstantProduct pool
pub fn migrate_to_v130(deps: DepsMut) -> Result<(), StdError> {
    #[cw_serde]
    pub struct PairInfoRawV120 {
        pub asset_infos: [AssetInfoRaw; 2],
        pub contract_addr: CanonicalAddr,
        pub liquidity_token: CanonicalAddr,
        pub asset_decimals: [u8; 2],
    }

    #[cw_serde]
    pub struct PairInfoRawV130 {
        pub asset_infos: [AssetInfoRaw; 2],
        pub contract_addr: CanonicalAddr,
        pub liquidity_token: CanonicalAddr,
        pub asset_decimals: [u8; 2],
        pub pair_type: PairType,
    }

    pub const PAIR_INFO_V120: Item<PairInfoRawV120> = Item::new("pair_info");
    pub const PAIR_INFO_V130: Item<PairInfoRawV130> = Item::new("pair_info");

    let config = PAIR_INFO_V120.load(deps.storage)?;
    PAIR_INFO_V130.save(
        deps.storage,
        &PairInfoRawV130 {
            asset_infos: config.asset_infos,
            contract_addr: config.contract_addr,
            liquidity_token: config.liquidity_token,
            asset_decimals: config.asset_decimals,
            pair_type: PairType::ConstantProduct,
        },
    )?;

    Ok(())
}
