use cosmwasm_std::{
    attr, entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, SubMsg, Uint128, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::{Cw20QueryMsg, MinterResponse, TokenInfoResponse};
use semver::Version;

use terraswap::asset::{Asset, AssetInfo};
use vault_network::vault::{
    Config, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, INSTANTIATE_LP_TOKEN_REPLY_ID,
};

use crate::{
    error::{StdResult, VaultError},
    execute::{callback, collect_protocol_fees, deposit, flash_loan, receive, update_config},
    queries::{get_config, get_protocol_fees, get_share},
    state::{ALL_TIME_COLLECTED_PROTOCOL_FEES, COLLECTED_PROTOCOL_FEES, CONFIG, LOAN_COUNTER},
};

const CONTRACT_NAME: &str = "vault_factory";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: deps.api.addr_validate(&msg.owner)?,
        asset_info: msg.asset_info.clone(),
        // we patch this in the INSTANTIATE_LP_TOKEN_REPLY
        liquidity_token: Addr::unchecked(""),
        fee_collector_addr: deps.api.addr_validate(&msg.fee_collector_addr)?,
        fees: msg.vault_fees,

        deposit_enabled: true,
        flash_loan_enabled: true,
        withdraw_enabled: true,
    };
    CONFIG.save(deps.storage, &config)?;

    // create the LP token for the vault
    let token_name = match &msg.asset_info {
        AssetInfo::NativeToken { denom } => denom.to_owned(),
        AssetInfo::Token { contract_addr } => {
            let token_info: TokenInfoResponse = deps
                .querier
                .query_wasm_smart(contract_addr, &Cw20QueryMsg::TokenInfo {})?;
            token_info.name
        }
    };

    // cw20 asset symbols are 3-12 characters,
    // so we take the first 8 characters of the symbol and append "uLP-" to it
    let lp_symbol = format!(
        "uLP-{}",
        msg.asset_info
            .clone()
            .get_label(&deps.as_ref())?
            .chars()
            .take(8)
            .collect::<String>()
    );

    let lp_label = format!(
        "WW Vault {} LP token",
        token_name.chars().take(32).collect::<String>()
    );

    let lp_instantiate_msg = SubMsg {
        id: INSTANTIATE_LP_TOKEN_REPLY_ID,
        gas_limit: None,
        reply_on: cosmwasm_std::ReplyOn::Success,
        msg: WasmMsg::Instantiate {
            admin: None,
            code_id: msg.token_id,
            msg: to_binary(&cw20_base::msg::InstantiateMsg {
                name: lp_label.clone(),
                symbol: lp_symbol,
                decimals: 6,
                initial_balances: vec![],
                mint: Some(MinterResponse {
                    minter: env.contract.address.to_string(),
                    cap: None,
                }),
                marketing: None,
            })?,
            funds: vec![],
            label: lp_label,
        }
        .into(),
    };

    // save protocol fee state
    COLLECTED_PROTOCOL_FEES.save(
        deps.storage,
        &Asset {
            amount: Uint128::zero(),
            info: msg.asset_info.clone(),
        },
    )?;
    ALL_TIME_COLLECTED_PROTOCOL_FEES.save(
        deps.storage,
        &Asset {
            amount: Uint128::zero(),
            info: msg.asset_info,
        },
    )?;

    // set loan counter to zero
    LOAN_COUNTER.save(deps.storage, &0)?;

    Ok(Response::new()
        .add_attributes(vec![attr("method", "instantiate")])
        .add_submessage(lp_instantiate_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateConfig(params) => update_config(deps, info, params),
        ExecuteMsg::Deposit { amount } => deposit(deps, env, info, amount),
        ExecuteMsg::FlashLoan { amount, msg } => flash_loan(deps, env, info, amount, msg),
        ExecuteMsg::CollectProtocolFees {} => collect_protocol_fees(deps),
        ExecuteMsg::Receive(msg) => receive(deps, env, info, msg),
        ExecuteMsg::Callback(msg) => callback(deps, env, info, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    // initialize the loan counter
    LOAN_COUNTER.save(deps.storage, &0)?;

    let version: Version = CONTRACT_VERSION
        .parse()
        .map_err(|_| StdError::parse_err("Version", "Failed to parse version"))?;
    let storage_version: Version = get_contract_version(deps.storage)?
        .version
        .parse()
        .map_err(|_| StdError::parse_err("Version", "Failed to parse storage_version"))?;

    if storage_version > version {
        return Err(VaultError::MigrateInvalidVersion {
            new_version: storage_version,
            current_version: version,
        });
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => get_config(deps),
        QueryMsg::Share { amount } => get_share(deps, env, amount),
        QueryMsg::ProtocolFees { all_time } => get_protocol_fees(deps, all_time),
    }
}
