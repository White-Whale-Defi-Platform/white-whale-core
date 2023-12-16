use crate::contract::{CREATE_PAIR_RESPONSE, CREATE_TRIO_RESPONSE};

use cosmwasm_std::{
    to_json_binary, wasm_execute, CosmosMsg, DepsMut, Env, MessageInfo, ReplyOn, Response, SubMsg,
    WasmMsg,
};

use white_whale::pool_network;
use white_whale::pool_network::asset::{AssetInfo, PairType};
use white_whale::pool_network::pair::{
    FeatureToggle, InstantiateMsg as PairInstantiateMsg, MigrateMsg as PairMigrateMsg, PoolFee,
};
use white_whale::pool_network::querier::query_balance;
use white_whale::pool_network::trio::{
    FeatureToggle as TrioFeatureToggle, InstantiateMsg as TrioInstantiateMsg,
    MigrateMsg as TrioMigrateMsg, PoolFee as TrioPoolFee, RampAmp,
};
use white_whale::pool_network::{pair, trio};

use crate::error::ContractError;
use crate::state::{
    add_allow_native_token, pair_key, trio_key, Config, TmpPairInfo, TmpTrioInfo, CONFIG, PAIRS,
    TMP_PAIR_INFO, TMP_TRIO_INFO, TRIOS,
};

/// Updates the contract's [Config]
pub fn update_config(
    deps: DepsMut,
    owner: Option<String>,
    fee_collector_addr: Option<String>,
    token_code_id: Option<u64>,
    pair_code_id: Option<u64>,
    trio_code_id: Option<u64>,
) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    if let Some(owner) = owner {
        // validate address format
        let _ = deps.api.addr_validate(&owner)?;

        config.owner = deps.api.addr_canonicalize(&owner)?;
    }

    if let Some(token_code_id) = token_code_id {
        config.token_code_id = token_code_id;
    }

    if let Some(pair_code_id) = pair_code_id {
        config.pair_code_id = pair_code_id;
    }

    if let Some(trio_code_id) = trio_code_id {
        config.trio_code_id = trio_code_id;
    }

    if let Some(fee_collector_addr) = fee_collector_addr {
        config.fee_collector_addr = deps.api.addr_validate(fee_collector_addr.as_str())?;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

/// Updates a pair config
pub fn update_pair_config(
    deps: DepsMut,
    pair_addr: String,
    owner: Option<String>,
    fee_collector_addr: Option<String>,
    pool_fees: Option<PoolFee>,
    feature_toggle: Option<FeatureToggle>,
) -> Result<Response, ContractError> {
    Ok(Response::new()
        .add_message(wasm_execute(
            deps.api.addr_validate(pair_addr.as_str())?.to_string(),
            &pool_network::pair::ExecuteMsg::UpdateConfig {
                owner,
                fee_collector_addr,
                pool_fees,
                feature_toggle,
            },
            vec![],
        )?)
        .add_attribute("action", "update_pair_config"))
}

/// Creates a Pair
pub fn create_pair(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset_infos: [AssetInfo; 2],
    pool_fees: PoolFee,
    pair_type: PairType,
    token_factory_lp: bool,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    if asset_infos[0] == asset_infos[1] {
        return Err(ContractError::SameAsset {});
    }

    let asset_1_decimal =
        match asset_infos[0].query_decimals(env.contract.address.clone(), &deps.querier) {
            Ok(decimal) => decimal,
            Err(_) => {
                return Err(ContractError::InvalidAsset {
                    asset: asset_infos[0].to_string(),
                });
            }
        };

    let asset_2_decimal =
        match asset_infos[1].query_decimals(env.contract.address.clone(), &deps.querier) {
            Ok(decimal) => decimal,
            Err(_) => {
                return Err(ContractError::InvalidAsset {
                    asset: asset_infos[1].to_string(),
                });
            }
        };

    let raw_infos = [
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
    ];

    let asset_decimals = [asset_1_decimal, asset_2_decimal];

    let pair_key = pair_key(&raw_infos);
    if let Ok(Some(_)) = PAIRS.may_load(deps.storage, &pair_key) {
        return Err(ContractError::ExistingPair {});
    }

    TMP_PAIR_INFO.save(
        deps.storage,
        &TmpPairInfo {
            pair_key,
            asset_infos: raw_infos,
            asset_decimals,
            pair_type: pair_type.clone(),
        },
    )?;

    // prepare labels for creating the pair token with a meaningful name
    let asset0_label = asset_infos[0].clone().get_label(&deps.as_ref())?;
    let asset1_label = asset_infos[1].clone().get_label(&deps.as_ref())?;
    let pair_label = format!("{asset0_label}-{asset1_label} pair");

    Ok(Response::new()
        .add_attributes(vec![
            ("action", "create_pair"),
            ("pair", &format!("{asset0_label}-{asset1_label}")),
            ("pair_label", pair_label.as_str()),
            ("pair_type", pair_type.get_label()),
        ])
        .add_submessage(SubMsg {
            id: CREATE_PAIR_RESPONSE,
            gas_limit: None,
            msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
                code_id: config.pair_code_id,
                funds: info.funds,
                admin: Some(env.contract.address.to_string()),
                label: pair_label,
                msg: to_json_binary(&PairInstantiateMsg {
                    asset_infos,
                    token_code_id: config.token_code_id,
                    asset_decimals,
                    pool_fees,
                    fee_collector_addr: config.fee_collector_addr.to_string(),
                    pair_type,
                    token_factory_lp,
                })?,
            }),
            reply_on: ReplyOn::Success,
        }))
}

/// Updates a trio config
pub fn update_trio_config(
    deps: DepsMut,
    trio_addr: String,
    owner: Option<String>,
    fee_collector_addr: Option<String>,
    pool_fees: Option<TrioPoolFee>,
    feature_toggle: Option<TrioFeatureToggle>,
    amp_factor: Option<RampAmp>,
) -> Result<Response, ContractError> {
    Ok(Response::new()
        .add_message(wasm_execute(
            deps.api.addr_validate(trio_addr.as_str())?.to_string(),
            &pool_network::trio::ExecuteMsg::UpdateConfig {
                owner,
                fee_collector_addr,
                pool_fees,
                feature_toggle,
                amp_factor,
            },
            vec![],
        )?)
        .add_attribute("action", "update_trio_config"))
}

/// Creates a Trio
pub fn create_trio(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset_infos: [AssetInfo; 3],
    pool_fees: TrioPoolFee,
    amp_factor: u64,
    token_factory_lp: bool,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    if (asset_infos[0] == asset_infos[1])
        || (asset_infos[0] == asset_infos[2])
        || (asset_infos[1] == asset_infos[2])
    {
        return Err(ContractError::SameAsset {});
    }

    let asset_1_decimal =
        match asset_infos[0].query_decimals(env.contract.address.clone(), &deps.querier) {
            Ok(decimal) => decimal,
            Err(_) => {
                return Err(ContractError::InvalidAsset {
                    asset: asset_infos[0].to_string(),
                });
            }
        };

    let asset_2_decimal =
        match asset_infos[1].query_decimals(env.contract.address.clone(), &deps.querier) {
            Ok(decimal) => decimal,
            Err(_) => {
                return Err(ContractError::InvalidAsset {
                    asset: asset_infos[1].to_string(),
                });
            }
        };
    let asset_3_decimal =
        match asset_infos[2].query_decimals(env.contract.address.clone(), &deps.querier) {
            Ok(decimal) => decimal,
            Err(_) => {
                return Err(ContractError::InvalidAsset {
                    asset: asset_infos[2].to_string(),
                });
            }
        };

    let raw_infos = [
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
        asset_infos[2].to_raw(deps.api)?,
    ];

    let asset_decimals = [asset_1_decimal, asset_2_decimal, asset_3_decimal];

    let trio_key = trio_key(&raw_infos);
    if let Ok(Some(_)) = TRIOS.may_load(deps.storage, &trio_key) {
        return Err(ContractError::ExistingTrio {});
    }

    TMP_TRIO_INFO.save(
        deps.storage,
        &TmpTrioInfo {
            trio_key,
            asset_infos: raw_infos,
            asset_decimals,
        },
    )?;

    // prepare labels for creating the pair token with a meaningful name
    let asset0_label = asset_infos[0].clone().get_label(&deps.as_ref())?;
    let asset1_label = asset_infos[1].clone().get_label(&deps.as_ref())?;
    let asset2_label = asset_infos[2].clone().get_label(&deps.as_ref())?;
    let trio_label = format!("{asset0_label}-{asset1_label}-{asset2_label} trio");

    Ok(Response::new()
        .add_attributes(vec![
            ("action", "create_trio"),
            (
                "trio",
                &format!("{asset0_label}-{asset1_label}-{asset2_label}"),
            ),
            ("trio_label", trio_label.as_str()),
        ])
        .add_submessage(SubMsg {
            id: CREATE_TRIO_RESPONSE,
            gas_limit: None,
            msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
                code_id: config.trio_code_id,
                funds: info.funds,
                admin: Some(env.contract.address.to_string()),
                label: trio_label,
                msg: to_json_binary(&TrioInstantiateMsg {
                    asset_infos,
                    token_code_id: config.token_code_id,
                    asset_decimals,
                    pool_fees,
                    fee_collector_addr: config.fee_collector_addr.to_string(),
                    amp_factor,
                    token_factory_lp,
                })?,
            }),
            reply_on: ReplyOn::Success,
        }))
}

pub fn remove_pair(
    deps: DepsMut,
    _env: Env,
    asset_infos: [AssetInfo; 2],
) -> Result<Response, ContractError> {
    let raw_infos = [
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
    ];

    let pair_key = pair_key(&raw_infos);
    let pair = PAIRS.may_load(deps.storage, &pair_key)?;

    let Some(pair) = pair else {
        return Err(ContractError::UnExistingPair {});
    };

    PAIRS.remove(deps.storage, &pair_key);

    Ok(Response::new().add_attributes(vec![
        ("action", "remove_pair"),
        (
            "pair_contract_addr",
            deps.api.addr_humanize(&pair.contract_addr)?.as_ref(),
        ),
    ]))
}

pub fn remove_trio(
    deps: DepsMut,
    _env: Env,
    asset_infos: [AssetInfo; 3],
) -> Result<Response, ContractError> {
    let raw_infos = [
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
        asset_infos[2].to_raw(deps.api)?,
    ];

    let trio_key = trio_key(&raw_infos);
    let trio = TRIOS.may_load(deps.storage, &trio_key)?;

    let Some(trio) = trio else {
        return Err(ContractError::NonExistantTrio {});
    };

    TRIOS.remove(deps.storage, &trio_key);

    Ok(Response::new().add_attributes(vec![
        ("action", "remove_trio"),
        (
            "trio_contract_addr",
            deps.api.addr_humanize(&trio.contract_addr)?.as_ref(),
        ),
    ]))
}

/// Adds native/ibc token with decimals to the factory's whitelist so it can create pairs with that asset
pub fn add_native_token_decimals(
    deps: DepsMut,
    env: Env,
    denom: String,
    decimals: u8,
) -> Result<Response, ContractError> {
    let balance = query_balance(&deps.querier, env.contract.address, denom.to_string())?;
    if balance.is_zero() {
        return Err(ContractError::InvalidVerificationBalance {});
    }

    add_allow_native_token(deps.storage, denom.to_string(), decimals)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "add_allow_native_token"),
        ("denom", &denom),
        ("decimals", &decimals.to_string()),
    ]))
}

/// Migrates a pair.
pub fn execute_migrate_pair(
    deps: DepsMut,
    contract: String,
    code_id: Option<u64>,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;
    let pair_code_id = code_id.unwrap_or(config.pair_code_id);

    let pool_response: pair::PoolResponse = deps
        .querier
        .query_wasm_smart(contract.as_str(), &pair::QueryMsg::Pool {})?;

    if pool_response.assets.len() != 2 {
        return Err(ContractError::MigratingWrongPool {});
    }

    Ok(
        Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Migrate {
            contract_addr: contract,
            new_code_id: pair_code_id,
            msg: to_json_binary(&PairMigrateMsg {})?,
        })),
    )
}

/// Migrates a trio.
pub fn execute_migrate_trio(
    deps: DepsMut,
    contract: String,
    code_id: Option<u64>,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;
    let trio_code_id = code_id.unwrap_or(config.trio_code_id);

    let trio_response: trio::PoolResponse = deps
        .querier
        .query_wasm_smart(contract.as_str(), &trio::QueryMsg::Pool {})?;

    if trio_response.assets.len() != 3 {
        return Err(ContractError::MigratingWrongPool {});
    }

    Ok(
        Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Migrate {
            contract_addr: contract,
            new_code_id: trio_code_id,
            msg: to_json_binary(&TrioMigrateMsg {})?,
        })),
    )
}
