use crate::state::{
    add_allow_native_token, pair_key, Config, TmpPairInfo, CONFIG, PAIRS, TMP_PAIR_INFO,
};
use cosmwasm_std::{
    to_binary, CosmosMsg, DepsMut, Env, MessageInfo, ReplyOn, Response, StdError, StdResult,
    SubMsg, WasmMsg,
};
use terraswap::asset::AssetInfo;
use terraswap::pair::{
    InstantiateMsg as PairInstantiateMsg, MigrateMsg as PairMigrateMsg, PoolFee,
};
use terraswap::querier::query_balance;

/// Updates the contract's [Config]
pub fn update_config(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    owner: Option<String>,
    fee_collector_addr: Option<String>,
    token_code_id: Option<u64>,
    pair_code_id: Option<u64>,
) -> StdResult<Response> {
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

/// Creates a Pair
pub fn create_pair(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    asset_infos: [AssetInfo; 2],
    pool_fees: PoolFee,
) -> StdResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;

    if asset_infos[0] == asset_infos[1] {
        return Err(StdError::generic_err("same asset"));
    }

    let asset_1_decimal =
        match asset_infos[0].query_decimals(env.contract.address.clone(), &deps.querier) {
            Ok(decimal) => decimal,
            Err(_) => return Err(StdError::generic_err("asset1 is invalid")),
        };

    let asset_2_decimal =
        match asset_infos[1].query_decimals(env.contract.address.clone(), &deps.querier) {
            Ok(decimal) => decimal,
            Err(_) => return Err(StdError::generic_err("asset2 is invalid")),
        };

    let raw_infos = [
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
    ];

    let asset_decimals = [asset_1_decimal, asset_2_decimal];

    let pair_key = pair_key(&raw_infos);
    if let Ok(Some(_)) = PAIRS.may_load(deps.storage, &pair_key) {
        return Err(StdError::generic_err("Pair already exists"));
    }

    TMP_PAIR_INFO.save(
        deps.storage,
        &TmpPairInfo {
            pair_key,
            asset_infos: raw_infos,
            asset_decimals,
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
                })?,
            }),
            reply_on: ReplyOn::Success,
        }))
}

/// Adds native/ibc token with decimals to the factory's whitelist so it can create pairs with that asset
pub fn add_native_token_decimals(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    denom: String,
    decimals: u8,
) -> StdResult<Response> {
    let _config: Config = CONFIG.load(deps.storage)?;

    let balance = query_balance(&deps.querier, env.contract.address, denom.to_string())?;
    if balance.is_zero() {
        return Err(StdError::generic_err(
            "a balance greater than zero is required by the factory for verification",
        ));
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
    _env: Env,
    _info: MessageInfo,
    contract: String,
    code_id: Option<u64>,
) -> StdResult<Response> {
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
