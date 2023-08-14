use classic_bindings::TerraQuery;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError,
    StdResult,
};
use cw2::{get_contract_version, set_contract_version};
use protobuf::Message;
use semver::Version;

use white_whale::pool_network::asset::{has_factory_token, AssetInfoRaw, PairInfoRaw};
use white_whale::pool_network::pair::{
    Config, ExecuteMsg, FeatureToggle, InstantiateMsg, MigrateMsg, QueryMsg,
};

use crate::error::ContractError;
use crate::response::MsgInstantiateContractResponse;
use crate::state::{
    ALL_TIME_BURNED_FEES, ALL_TIME_COLLECTED_PROTOCOL_FEES, COLLECTED_PROTOCOL_FEES, CONFIG,
    PAIR_INFO,
};
use crate::{commands, helpers, queries};

// version info for migration info
const CONTRACT_NAME: &str = "white_whale-pool";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const INSTANTIATE_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<TerraQuery>,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    if has_factory_token(&msg.asset_infos) && msg.pool_fees.burn_fee.share > Decimal::zero() {
        return Err(ContractError::TokenFactoryAssetBurnDisabled {});
    }

    let pair_info: &PairInfoRaw = &PairInfoRaw {
        contract_addr: deps.api.addr_canonicalize(env.contract.address.as_str())?,
        liquidity_token: AssetInfoRaw::NativeToken {
            denom: "".to_string(),
        },
        asset_infos: [
            msg.asset_infos[0].to_raw(deps.api)?,
            msg.asset_infos[1].to_raw(deps.api)?,
        ],
        asset_decimals: msg.asset_decimals,
        pair_type: msg.pair_type.clone(),
    };

    PAIR_INFO.save(deps.storage, pair_info)?;

    let asset_info_0 = pair_info.asset_infos[0].to_normal(deps.api)?;
    let asset_info_1 = pair_info.asset_infos[1].to_normal(deps.api)?;

    let asset0_label = asset_info_0.clone().get_label(&deps.as_ref())?;
    let asset1_label = asset_info_1.clone().get_label(&deps.as_ref())?;
    let lp_token_name = format!("{asset0_label}-{asset1_label}-LP");

    // check the fees are valid
    msg.pool_fees.is_valid()?;

    // Set owner and initial pool fees
    let config = Config {
        owner: deps.api.addr_validate(info.sender.as_str())?,
        fee_collector_addr: deps.api.addr_validate(msg.fee_collector_addr.as_str())?,
        pool_fees: msg.pool_fees.clone(),
        feature_toggle: FeatureToggle {
            withdrawals_enabled: true,
            deposits_enabled: true,
            swaps_enabled: true,
        },
    };
    CONFIG.save(deps.storage, &config)?;

    // Instantiate the collected protocol fees
    helpers::instantiate_fees(
        deps.storage,
        asset_info_0.clone(),
        asset_info_1.clone(),
        COLLECTED_PROTOCOL_FEES,
    )?;
    helpers::instantiate_fees(
        deps.storage,
        asset_info_0.clone(),
        asset_info_1.clone(),
        ALL_TIME_COLLECTED_PROTOCOL_FEES,
    )?;
    helpers::instantiate_fees(
        deps.storage,
        asset_info_0,
        asset_info_1,
        ALL_TIME_BURNED_FEES,
    )?;

    helpers::create_lp_token(deps, &env, &msg, &lp_token_name)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<TerraQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => commands::receive_cw20(deps, env, info, msg),
        ExecuteMsg::ProvideLiquidity {
            assets,
            slippage_tolerance,
            receiver,
        } => commands::provide_liquidity(deps, env, info, assets, slippage_tolerance, receiver),
        ExecuteMsg::WithdrawLiquidity {} => {
            // validate that the asset sent is the token factory LP token
            let pair_info = PAIR_INFO.load(deps.storage)?;
            let lp_token_denom = match pair_info.liquidity_token {
                AssetInfoRaw::Token { .. } => String::new(),
                AssetInfoRaw::NativeToken { denom } => denom,
            };

            if info.funds.len() != 1 || info.funds[0].denom != lp_token_denom {
                return Err(ContractError::AssetMismatch {});
            }

            commands::withdraw_liquidity(deps, env, info.sender, info.funds[0].amount)
        }
        ExecuteMsg::Swap {
            offer_asset,
            belief_price,
            max_spread,
            to,
        } => {
            // check if the swap feature is enabled
            let feature_toggle: FeatureToggle = CONFIG.load(deps.storage)?.feature_toggle;
            if !feature_toggle.swaps_enabled {
                return Err(ContractError::OperationDisabled("swap".to_string()));
            }

            if !offer_asset.is_native_token() {
                return Err(ContractError::Unauthorized {});
            }

            let to_addr = if let Some(to_addr) = to {
                Some(deps.api.addr_validate(&to_addr)?)
            } else {
                None
            };

            commands::swap(
                deps,
                env,
                info.clone(),
                info.sender,
                offer_asset,
                belief_price,
                max_spread,
                to_addr,
            )
        }
        ExecuteMsg::UpdateConfig {
            owner,
            fee_collector_addr,
            pool_fees,
            feature_toggle,
        } => commands::update_config(
            deps,
            info,
            owner,
            fee_collector_addr,
            pool_fees,
            feature_toggle,
        ),
        ExecuteMsg::CollectProtocolFees {} => commands::collect_protocol_fees(deps),
    }
}

/// This just stores the result for future query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut<TerraQuery>, _env: Env, msg: Reply) -> StdResult<Response> {
    let data = msg.result.unwrap().data.unwrap();
    let res: MsgInstantiateContractResponse =
        Message::parse_from_bytes(data.as_slice()).map_err(|_| {
            StdError::parse_err("MsgInstantiateContractResponse", "failed to parse data")
        })?;
    let liquidity_token = res.address;

    let api = deps.api;
    PAIR_INFO.update(deps.storage, |mut meta| -> StdResult<_> {
        meta.liquidity_token = AssetInfoRaw::Token {
            contract_addr: api.addr_canonicalize(&liquidity_token)?,
        };
        Ok(meta)
    })?;

    Ok(Response::new().add_attribute("liquidity_token_addr", liquidity_token))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<TerraQuery>, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Pair {} => Ok(to_binary(&queries::query_pair_info(deps)?)?),
        QueryMsg::Pool {} => Ok(to_binary(&queries::query_pool(deps)?)?),
        QueryMsg::Simulation { offer_asset } => {
            Ok(to_binary(&queries::query_simulation(deps, offer_asset)?)?)
        }
        QueryMsg::ReverseSimulation { ask_asset } => Ok(to_binary(
            &queries::query_reverse_simulation(deps, ask_asset)?,
        )?),
        QueryMsg::Config {} => Ok(to_binary(&queries::query_config(deps)?)?),
        QueryMsg::ProtocolFees { asset_id, all_time } => Ok(to_binary(&queries::query_fees(
            deps,
            asset_id,
            all_time,
            COLLECTED_PROTOCOL_FEES,
            Some(ALL_TIME_COLLECTED_PROTOCOL_FEES),
        )?)?),
        QueryMsg::BurnedFees { asset_id } => Ok(to_binary(&queries::query_fees(
            deps,
            asset_id,
            None,
            ALL_TIME_BURNED_FEES,
            None,
        )?)?),
    }
}

#[cfg(not(tarpaulin_include))]
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(
    mut deps: DepsMut<TerraQuery>,
    _env: Env,
    _msg: MigrateMsg,
) -> Result<Response, ContractError> {
    use white_whale::migrate_guards::check_contract_name;

    use crate::migrations;

    check_contract_name(deps.storage, CONTRACT_NAME.to_string())?;

    let version: Version = CONTRACT_VERSION.parse()?;
    let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;

    if storage_version >= version {
        return Err(ContractError::MigrateInvalidVersion {
            current_version: storage_version,
            new_version: version,
        });
    }

    #[cfg(feature = "injective")]
    if storage_version <= Version::parse("1.1.0")? {
        migrations::migrate_to_v13x(deps.branch())?;
    }

    #[cfg(not(feature = "injective"))]
    if storage_version <= Version::parse("1.0.4")? {
        migrations::migrate_to_v110(deps.branch())?;
    } else if storage_version == Version::parse("1.1.0")? {
        migrations::migrate_to_v120(deps.branch())?;
    }
    #[cfg(not(feature = "injective"))]
    if storage_version == Version::parse("1.2.0")? {
        migrations::migrate_to_v130(deps.branch())?;
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
