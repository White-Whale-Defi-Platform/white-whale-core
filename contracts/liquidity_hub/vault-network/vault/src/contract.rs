use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, SubMsg, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::{Cw20QueryMsg, MinterResponse, TokenInfoResponse};
use semver::Version;
use terraswap::asset::AssetInfo;
use vault_network::vault::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, INSTANTIATE_LP_TOKEN_REPLY_ID,
};

use crate::{
    execute::{callback, deposit, flash_loan, receive, update_config},
    queries::{get_config, get_share},
    state::{Config, CONFIG},
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

    Ok(Response::default().add_submessage(lp_instantiate_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateConfig {
            flash_loan_enabled,
            withdraw_enabled,
            deposit_enabled,
            new_owner,
        } => update_config(
            deps,
            info,
            flash_loan_enabled,
            withdraw_enabled,
            deposit_enabled,
            new_owner,
        ),
        ExecuteMsg::Deposit { amount } => deposit(deps, env, info, amount),
        ExecuteMsg::FlashLoan { amount, msg } => flash_loan(deps, env, info, amount, msg),
        ExecuteMsg::Receive(msg) => receive(deps, env, info, msg),
        ExecuteMsg::Callback(msg) => callback(deps, env, info, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    let version: Version = CONTRACT_VERSION
        .parse()
        .map_err(|_| StdError::parse_err("Version", "Failed to parse version"))?;
    let storage_version: Version = get_contract_version(deps.storage)?
        .version
        .parse()
        .map_err(|_| StdError::parse_err("Version", "Failed to parse storage_version"))?;

    if storage_version > version {
        return Err(StdError::generic_err(format!(
            "Attempt to migrate to version \"{}\" which is lower than current version \"{}\"",
            storage_version, version
        )));
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => get_config(deps),
        QueryMsg::Share { amount } => get_share(deps, env, amount),
    }
}
