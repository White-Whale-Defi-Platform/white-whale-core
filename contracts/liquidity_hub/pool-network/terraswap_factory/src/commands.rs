use cosmwasm_std::{
    to_binary, wasm_execute, CosmosMsg, DepsMut, Env, ReplyOn, Response, SubMsg, WasmMsg,
};

use pool_network::asset::{AssetInfo, PairType};
use pool_network::pair::{
    FeatureToggle, InstantiateMsg as PairInstantiateMsg, MigrateMsg as PairMigrateMsg, PoolFee,
};
use pool_network::querier::query_balance;

use crate::error::ContractError;
use crate::state::{
    add_allow_native_token, pair_key, Config, TmpPairInfo, CONFIG, PAIRS, TMP_PAIR_INFO,
};

/// Updates the contract's [Config]
pub fn update_config(
    deps: DepsMut,
    owner: Option<String>,
    fee_collector_addr: Option<String>,
    token_code_id: Option<u64>,
    pair_code_id: Option<u64>,
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
    asset_infos: [AssetInfo; 2],
    pool_fees: PoolFee,
    pair_type: PairType,
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
    let pair_label = format!("{}-{} pair", asset0_label, asset1_label);

    Ok(Response::new()
        .add_attributes(vec![
            ("action", "create_pair"),
            ("pair", &format!("{}-{}", asset0_label, asset1_label)),
            ("pair_label", pair_label.as_str()),
            ("pair_type", pair_type.get_label()),
        ])
        .add_submessage(SubMsg {
            id: 1,
            gas_limit: None,
            msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
                code_id: config.pair_code_id,
                funds: vec![],
                admin: Some(env.contract.address.to_string()),
                label: pair_label,
                msg: to_binary(&PairInstantiateMsg {
                    asset_infos,
                    token_code_id: config.token_code_id,
                    asset_decimals,
                    pool_fees,
                    fee_collector_addr: config.fee_collector_addr.to_string(),
                    pair_type,
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

pub fn execute_migrate_pair(
    deps: DepsMut,
    contract: String,
    code_id: Option<u64>,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;
    let code_id = code_id.unwrap_or(config.pair_code_id);

    Ok(
        Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Migrate {
            contract_addr: contract,
            new_code_id: code_id,
            msg: to_binary(&PairMigrateMsg {})?,
        })),
    )
}
