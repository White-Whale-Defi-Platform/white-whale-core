use classic_bindings::TerraQuery;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError, StdResult,
};
use cw2::{get_contract_version, set_contract_version};
use protobuf::Message;
use semver::Version;

use white_whale_std::pool_network::asset::{AssetInfoRaw, TrioInfoRaw};
use white_whale_std::pool_network::trio::{
    Config, ExecuteMsg, FeatureToggle, InstantiateMsg, MigrateMsg, QueryMsg,
};

use crate::error::ContractError;
use crate::error::ContractError::MigrateInvalidVersion;
use crate::response::MsgInstantiateContractResponse;
use crate::state::{
    ALL_TIME_BURNED_FEES, ALL_TIME_COLLECTED_PROTOCOL_FEES, COLLECTED_PROTOCOL_FEES, CONFIG,
    TRIO_INFO,
};
use crate::{commands, helpers, queries};

// version info for migration info
const CONTRACT_NAME: &str = "white_whale-stableswap-3pool";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const INSTANTIATE_REPLY_ID: u64 = 1;

/// Minimum amplification coefficient.
pub const MIN_AMP: u64 = 1;
/// Maximum amplification coefficient.
pub const MAX_AMP: u64 = 1_000_000;
/// Minimum number of blocks an amplification coefficient change must take place over.
pub const MIN_RAMP_BLOCKS: u64 = 10000;
/// Maximum factor the amplification coefficient can be changed by in a single command.
pub const MAX_AMP_CHANGE: u64 = 10;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<TerraQuery>,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let trio_info: &TrioInfoRaw = &TrioInfoRaw {
        contract_addr: deps.api.addr_canonicalize(env.contract.address.as_str())?,
        liquidity_token: AssetInfoRaw::NativeToken {
            denom: "".to_string(),
        },
        asset_infos: [
            msg.asset_infos[0].to_raw(deps.api)?,
            msg.asset_infos[1].to_raw(deps.api)?,
            msg.asset_infos[2].to_raw(deps.api)?,
        ],
        asset_decimals: msg.asset_decimals,
    };

    TRIO_INFO.save(deps.storage, trio_info)?;

    let asset_info_0 = trio_info.asset_infos[0].to_normal(deps.api)?;
    let asset_info_1 = trio_info.asset_infos[1].to_normal(deps.api)?;
    let asset_info_2 = trio_info.asset_infos[2].to_normal(deps.api)?;

    let asset0_label = asset_info_0.clone().get_label(&deps.as_ref())?;
    let asset1_label = asset_info_1.clone().get_label(&deps.as_ref())?;
    let asset2_label = asset_info_2.clone().get_label(&deps.as_ref())?;
    let lp_token_name = format!("{asset0_label}-{asset1_label}-{asset2_label}-LP");

    // check the fees are valid
    msg.pool_fees.is_valid()?;
    //check initial amp is in range
    if msg.amp_factor < MIN_AMP {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Initial amp must be over {MIN_AMP}"
        ))));
    }
    if msg.amp_factor > MAX_AMP {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Initial amp must be under {MAX_AMP}",
        ))));
    }
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
        initial_amp: msg.amp_factor,
        future_amp: msg.amp_factor,
        initial_amp_block: env.block.height,
        future_amp_block: env.block.height,
    };

    CONFIG.save(deps.storage, &config)?;

    // Instantiate the collected protocol fees
    helpers::instantiate_fees(
        deps.storage,
        asset_info_0.clone(),
        asset_info_1.clone(),
        asset_info_2.clone(),
        COLLECTED_PROTOCOL_FEES,
    )?;
    helpers::instantiate_fees(
        deps.storage,
        asset_info_0.clone(),
        asset_info_1.clone(),
        asset_info_2.clone(),
        ALL_TIME_COLLECTED_PROTOCOL_FEES,
    )?;
    helpers::instantiate_fees(
        deps.storage,
        asset_info_0,
        asset_info_1,
        asset_info_2,
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
            let trio_info = TRIO_INFO.load(deps.storage)?;
            let lp_token_denom = match trio_info.liquidity_token {
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
            ask_asset,
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
                ask_asset,
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
            amp_factor,
        } => commands::update_config(
            deps,
            env,
            info,
            owner,
            fee_collector_addr,
            pool_fees,
            feature_toggle,
            amp_factor,
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
    TRIO_INFO.update(deps.storage, |mut meta| -> StdResult<_> {
        meta.liquidity_token = AssetInfoRaw::Token {
            contract_addr: api.addr_canonicalize(&liquidity_token)?,
        };
        Ok(meta)
    })?;

    Ok(Response::new().add_attribute("liquidity_token_addr", liquidity_token))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<TerraQuery>, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Trio {} => Ok(to_json_binary(&queries::query_trio_info(deps)?)?),
        QueryMsg::Pool {} => Ok(to_json_binary(&queries::query_pool(deps)?)?),
        QueryMsg::Simulation {
            offer_asset,
            ask_asset,
        } => Ok(to_json_binary(&queries::query_simulation(
            deps,
            offer_asset,
            ask_asset,
            env.block.height,
        )?)?),
        QueryMsg::ReverseSimulation {
            ask_asset,
            offer_asset,
        } => Ok(to_json_binary(&queries::query_reverse_simulation(
            deps,
            ask_asset,
            offer_asset,
            env.block.height,
        )?)?),
        QueryMsg::Config {} => Ok(to_json_binary(&queries::query_config(deps)?)?),
        QueryMsg::ProtocolFees { asset_id, all_time } => Ok(to_json_binary(&queries::query_fees(
            deps,
            asset_id,
            all_time,
            COLLECTED_PROTOCOL_FEES,
            Some(ALL_TIME_COLLECTED_PROTOCOL_FEES),
        )?)?),
        QueryMsg::BurnedFees { asset_id } => Ok(to_json_binary(&queries::query_fees(
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
    deps: DepsMut<TerraQuery>,
    _env: Env,
    _msg: MigrateMsg,
) -> Result<Response, ContractError> {
    use white_whale_std::migrate_guards::check_contract_name;

    check_contract_name(deps.storage, CONTRACT_NAME.to_string())?;

    let version: Version = CONTRACT_VERSION.parse()?;
    let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;

    if storage_version >= version {
        return Err(MigrateInvalidVersion {
            current_version: storage_version,
            new_version: version,
        });
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
