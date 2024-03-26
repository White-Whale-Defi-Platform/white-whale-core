#![cfg(not(tarpaulin_include))]
use cosmwasm_schema::cw_serde;
#[cfg(all(not(feature = "injective"), not(feature = "osmosis")))]
use cosmwasm_std::Uint128;
use cosmwasm_std::{Addr, DepsMut, StdError};
#[cfg(not(feature = "osmosis"))]
use cosmwasm_std::{CanonicalAddr, Decimal};
use cw_storage_plus::Item;
#[cfg(all(not(feature = "injective"), not(feature = "osmosis")))]
use schemars::JsonSchema;
#[cfg(all(not(feature = "injective"), not(feature = "osmosis")))]
use serde::{Deserialize, Serialize};

#[cfg(all(not(feature = "injective"), not(feature = "osmosis")))]
use crate::state::PAIR_INFO;

#[cfg(not(feature = "osmosis"))]
use white_whale_std::fee::Fee;
use white_whale_std::pool_network;
#[cfg(not(feature = "osmosis"))]
use white_whale_std::pool_network::asset::{AssetInfo, AssetInfoRaw, PairType};
use white_whale_std::pool_network::pair::{Config, FeatureToggle};

#[cfg(not(feature = "osmosis"))]
use crate::helpers::instantiate_fees;
#[cfg(not(feature = "osmosis"))]
use crate::state::ALL_TIME_BURNED_FEES;
use crate::state::CONFIG;

#[cfg(all(not(feature = "injective"), not(feature = "osmosis")))]
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

#[cfg(all(not(feature = "injective"), not(feature = "osmosis")))]
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

#[cfg(all(not(feature = "injective"), not(feature = "osmosis")))]
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
        pub liquidity_token: AssetInfoRaw,
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
            // all liquidity tokens until this version are cw20 tokens
            liquidity_token: AssetInfoRaw::Token {
                contract_addr: config.liquidity_token,
            },
            asset_decimals: config.asset_decimals,
            // all pools until this version are ConstantProduct
            pair_type: PairType::ConstantProduct,
        },
    )?;

    Ok(())
}

#[cfg(feature = "injective")]
/// Migrates the state from v1.1.0 to v1.3.x in a single migration
pub fn migrate_to_v13x(deps: DepsMut) -> Result<(), StdError> {
    // migration to v1.2.0
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

    const CONFIG_V110: Item<ConfigV110> = Item::new("leaderboard");
    let leaderboard = CONFIG_V110.load(deps.storage)?;

    let mut start_from: Option<String> = None;
    for (addr, amount) in leaderboard.iter() {
        let leaderboard = deps.api.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: "guppy_furnace".to_string(),
            msg: to_binary(&LeaderBoard {
                start_from: start_from,
                limit: 30,
            })?,
        }))?;

        LEADERBOARD.save(deps.storage, &"uguppy", &leaderboard)?;

        start_from = Some(leaderboard.last()?);
    }

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

    #[cw_serde]
    struct PairInfoRawV110 {
        pub asset_infos: [AssetInfoRaw; 2],
        pub contract_addr: CanonicalAddr,
        pub liquidity_token: CanonicalAddr,
        pub asset_decimals: [u8; 2],
    }

    #[cw_serde]
    struct PairInfoV110 {
        pub asset_infos: [AssetInfo; 2],
        pub contract_addr: String,
        pub liquidity_token: String,
        pub asset_decimals: [u8; 2],
    }

    const PAIR_INFO_V110: Item<PairInfoRawV110> = Item::new("pair_info");

    // Instantiates the ALL_TIME_BURNED_FEES
    let pair_info = PAIR_INFO_V110.load(deps.storage)?;
    let asset_info_0 = pair_info.asset_infos[0].to_normal(deps.api)?;
    let asset_info_1 = pair_info.asset_infos[1].to_normal(deps.api)?;

    instantiate_fees(
        deps.storage,
        asset_info_0,
        asset_info_1,
        ALL_TIME_BURNED_FEES,
    )?;

    // migration to v1.2.0
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
        pub liquidity_token: AssetInfoRaw,
        pub asset_decimals: [u8; 2],
        pub pair_type: PairType,
    }

    const PAIR_INFO_V120: Item<PairInfoRawV120> = Item::new("pair_info");
    const PAIR_INFO_V130: Item<PairInfoRawV130> = Item::new("pair_info");

    let config = PAIR_INFO_V120.load(deps.storage)?;
    PAIR_INFO_V130.save(
        deps.storage,
        &PairInfoRawV130 {
            asset_infos: config.asset_infos,
            contract_addr: config.contract_addr,
            // all liquidity tokens until this version are cw20 tokens
            liquidity_token: AssetInfoRaw::Token {
                contract_addr: config.liquidity_token,
            },
            asset_decimals: config.asset_decimals,
            // all pools until this version are ConstantProduct
            pair_type: PairType::ConstantProduct,
        },
    )?;

    Ok(())
}

/// This migration adds the `cosmwasm_pool_interface` to the config, so we can see if the swap is coming from
/// the osmosis pool manager or not in order to pay the osmosis taker fee.
#[cfg(feature = "osmosis")]
pub fn migrate_to_v135(deps: DepsMut) -> Result<(), StdError> {
    #[cw_serde]
    struct ConfigV133 {
        pub owner: Addr,
        pub fee_collector_addr: Addr,
        pub pool_fees: pool_network::pair::PoolFee,
        pub feature_toggle: FeatureToggle,
    }

    const CONFIG_V133: Item<ConfigV133> = Item::new("config");
    let config_v133 = CONFIG_V133.load(deps.storage)?;

    // Add burn fee to config. Zero fee is used as default.
    let config = Config {
        owner: config_v133.owner,
        fee_collector_addr: config_v133.fee_collector_addr,
        pool_fees: config_v133.pool_fees,
        feature_toggle: config_v133.feature_toggle,
        // set the cosmwasm pool interface to empty for now
        cosmwasm_pool_interface: Addr::unchecked(""),
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(())
}
