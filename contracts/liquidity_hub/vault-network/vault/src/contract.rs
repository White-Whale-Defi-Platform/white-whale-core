use cosmwasm_std::{
    attr, entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, SubMsg, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::MinterResponse;
use semver::Version;

use pool_network::asset::IBC_PREFIX;
#[cfg(feature = "injective")]
use pool_network::asset::PEGGY_PREFIX;
use vault_network::vault::{
    Config, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, INSTANTIATE_LP_TOKEN_REPLY_ID,
};

use crate::state::{initialize_fee, ALL_TIME_BURNED_FEES};
use crate::{
    error::VaultError,
    execute::{callback, collect_protocol_fees, deposit, flash_loan, receive, update_config},
    migrations,
    queries::{get_config, get_fees, get_payback_amount, get_share},
    state::{ALL_TIME_COLLECTED_PROTOCOL_FEES, COLLECTED_PROTOCOL_FEES, CONFIG, LOAN_COUNTER},
};

const CONTRACT_NAME: &str = "white_whale-vault";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, VaultError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // check the fees are valid
    msg.vault_fees.is_valid()?;

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

    let asset_label: String = msg.asset_info.clone().get_label(&deps.as_ref())?;

    // cw20 asset symbols are 3-12 characters,
    // so we take the first 8 characters of the symbol and append "uLP-" to it
    let mut lp_symbol = format!("uLP-{}", asset_label.chars().take(8).collect::<String>());

    // in case it's an ibc token, strip away everything from the '/' in 'ibc/'. The resulting
    // lp_symbol would be uLP-ibc in that case.
    if asset_label.starts_with(IBC_PREFIX) {
        lp_symbol = lp_symbol.splitn(2, '/').collect::<Vec<_>>()[0].to_string();
    }

    #[cfg(feature = "injective")]
    {
        // in case it is an Ethereum bridged (peggy) asset on Injective, strip away everything from
        // the "0x" in 'peggy0x...'. The resulting lp_symbol would be uLP-peggy in that case.
        if asset_label.starts_with(PEGGY_PREFIX) {
            lp_symbol = lp_symbol.splitn(2, "0x").collect::<Vec<_>>()[0].to_string();
        }
    }

    let lp_label = format!(
        "WW Vault {} LP token",
        msg.asset_info
            .clone()
            .get_label(&deps.as_ref())?
            .chars()
            .take(32)
            .collect::<String>()
    );

    let lp_instantiate_msg = SubMsg {
        id: INSTANTIATE_LP_TOKEN_REPLY_ID,
        gas_limit: None,
        reply_on: cosmwasm_std::ReplyOn::Success,
        msg: WasmMsg::Instantiate {
            admin: None,
            code_id: msg.token_id,
            msg: to_binary(&pool_network::token::InstantiateMsg {
                name: lp_label.clone(),
                symbol: lp_symbol,
                decimals: 6,
                initial_balances: vec![],
                mint: Some(MinterResponse {
                    minter: env.contract.address.to_string(),
                    cap: None,
                }),
            })?,
            funds: vec![],
            label: lp_label,
        }
        .into(),
    };

    // initialize fees in state
    initialize_fee(
        deps.storage,
        COLLECTED_PROTOCOL_FEES,
        msg.asset_info.clone(),
    )?;
    initialize_fee(
        deps.storage,
        ALL_TIME_COLLECTED_PROTOCOL_FEES,
        msg.asset_info.clone(),
    )?;
    initialize_fee(deps.storage, ALL_TIME_BURNED_FEES, msg.asset_info)?;

    // set loan counter to zero
    LOAN_COUNTER.save(deps.storage, &0)?;

    Ok(Response::new()
        .add_attributes(vec![attr("method", "instantiate")])
        .add_submessage(lp_instantiate_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, VaultError> {
    match msg {
        ExecuteMsg::UpdateConfig(params) => update_config(deps, info, params),
        ExecuteMsg::Deposit { amount } => deposit(deps, env, info, amount),
        ExecuteMsg::FlashLoan { amount, msg } => flash_loan(deps, env, info, amount, msg),
        ExecuteMsg::CollectProtocolFees {} => collect_protocol_fees(deps),
        ExecuteMsg::Receive(msg) => receive(deps, env, info, msg),
        ExecuteMsg::Callback(msg) => callback(deps, env, info, msg),
    }
}

#[cfg(not(tarpaulin_include))]
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(mut deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, VaultError> {
    // initialize the loan counter
    LOAN_COUNTER.save(deps.storage, &0)?;

    let version: Version = CONTRACT_VERSION
        .parse()
        .map_err(|_| StdError::parse_err("Version", "Failed to parse version"))?;
    let storage_version: Version = get_contract_version(deps.storage)?
        .version
        .parse()
        .map_err(|_| StdError::parse_err("Version", "Failed to parse storage_version"))?;

    if storage_version >= version {
        return Err(VaultError::MigrateInvalidVersion {
            new_version: storage_version,
            current_version: version,
        });
    }

    if storage_version
        <= Version::parse("1.1.3")
            .map_err(|_| StdError::parse_err("Version", "Failed to parse version"))?
    {
        migrations::migrate_to_v120(deps.branch())?;
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, VaultError> {
    match msg {
        QueryMsg::Config {} => get_config(deps),
        QueryMsg::Share { amount } => get_share(deps, env, amount),
        QueryMsg::ProtocolFees { all_time } => get_fees(
            deps,
            all_time,
            ALL_TIME_COLLECTED_PROTOCOL_FEES,
            Some(COLLECTED_PROTOCOL_FEES),
        ),
        QueryMsg::GetPaybackAmount { amount } => get_payback_amount(deps, amount),
        QueryMsg::BurnedFees {} => get_fees(deps, true, ALL_TIME_BURNED_FEES, None),
    }
}
