use cosmwasm_std::{entry_point, to_json_binary};
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response};
use cw2::{get_contract_version, set_contract_version};

use semver::Version;

use white_whale_std::vault_manager::{Config, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

use crate::error::ContractError;

use crate::state::{CONFIG, ONGOING_FLASHLOAN, VAULT_COUNTER};
use crate::{manager, queries, router, vault};

// version info for migration info
const CONTRACT_NAME: &str = "white-whale_vault-manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        whale_lair_addr: deps.api.addr_validate(&msg.whale_lair_addr)?,
        vault_creation_fee: msg.vault_creation_fee.clone(),
        flash_loan_enabled: true,
        deposit_enabled: true,
        withdraw_enabled: true,
    };

    CONFIG.save(deps.storage, &config)?;

    cw_ownable::initialize_owner(deps.storage, deps.api, Some(msg.owner.as_str()))?;

    // set flashloan counter to false
    ONGOING_FLASHLOAN.save(deps.storage, &false)?;
    // initialize vault counter
    VAULT_COUNTER.save(deps.storage, &0u64)?;

    Ok(Response::default().add_attributes(vec![
        ("action", "instantiate".to_string()),
        ("owner", msg.owner),
        ("whale_lair_addr", config.whale_lair_addr.into_string()),
        ("vault_creation_fee", config.vault_creation_fee.to_string()),
    ]))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateVault {
            asset_denom,
            fees,
            vault_identifier,
        } => manager::commands::create_vault(deps, env, info, asset_denom, fees, vault_identifier),
        ExecuteMsg::UpdateConfig {
            whale_lair_addr,
            vault_creation_fee,
            flash_loan_enabled,
            deposit_enabled,
            withdraw_enabled,
        } => manager::commands::update_config(
            deps,
            info,
            whale_lair_addr,
            vault_creation_fee,
            flash_loan_enabled,
            deposit_enabled,
            withdraw_enabled,
        ),
        ExecuteMsg::Deposit { vault_identifier } => {
            vault::commands::deposit(deps, &env, &info, &vault_identifier)
        }
        ExecuteMsg::Withdraw => vault::commands::withdraw(deps, &env, &info),
        ExecuteMsg::FlashLoan {
            asset,
            vault_identifier,
            payload,
        } => router::commands::flash_loan(deps, env, info, asset, vault_identifier, payload),
        ExecuteMsg::Callback(msg) => router::commands::callback(deps, env, info, msg),
        ExecuteMsg::UpdateOwnership(action) => {
            cw_utils::nonpayable(&info)?;
            white_whale_std::common::update_ownership(deps, env, info, action).map_err(Into::into)
        }
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Config {} => Ok(to_json_binary(&queries::query_manager_config(deps)?)?),
        QueryMsg::Vault { filter_by } => {
            Ok(to_json_binary(&queries::query_vault(deps, filter_by)?)?)
        }
        QueryMsg::Vaults { start_after, limit } => Ok(to_json_binary(&queries::query_vaults(
            deps,
            start_after,
            limit,
        )?)?),
        QueryMsg::Share { lp_share } => Ok(to_json_binary(&queries::get_share(deps, lp_share)?)?),
        QueryMsg::PaybackAmount {
            asset,
            vault_identifier,
        } => Ok(to_json_binary(&queries::get_payback_amount(
            deps,
            asset,
            vault_identifier,
        )?)?),
        QueryMsg::Ownership {} => Ok(to_json_binary(&cw_ownable::get_ownership(deps.storage)?)?),
    }
}

#[cfg(not(tarpaulin_include))]
#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    use white_whale_std::migrate_guards::check_contract_name;

    check_contract_name(deps.storage, CONTRACT_NAME.to_string())?;

    let version: Version = CONTRACT_VERSION.parse()?;
    let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;

    if storage_version >= version {
        return Err(ContractError::MigrateInvalidVersion {
            current_version: storage_version,
            new_version: version,
        });
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
