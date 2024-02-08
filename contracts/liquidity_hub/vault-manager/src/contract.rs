use cosmwasm_std::{entry_point, from_json, to_json_binary};
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError};
use cw2::{get_contract_version, set_contract_version};
use cw_utils::one_coin;
use semver::Version;

use white_whale_std::pool_network::asset::AssetInfo;
use white_whale_std::vault_manager::{
    Config, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};

use crate::error::ContractError;
use crate::helpers::assert_asset;
use crate::state::{get_vault_by_lp, CONFIG, ONGOING_FLASHLOAN, VAULT_COUNTER};
use crate::{manager, queries, router, vault};

// version info for migration info
const CONTRACT_NAME: &str = "ww-vault-manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    match msg.vault_creation_fee.info {
        AssetInfo::Token { .. } => {
            return Err(StdError::generic_err("Vault creation fee must be native token").into());
        }
        AssetInfo::NativeToken { .. } => {}
    }

    let config = Config {
        lp_token_type: msg.lp_token_type,
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
        ("lp_token_type", config.lp_token_type.to_string()),
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
            asset_info,
            fees,
            vault_identifier,
        } => manager::commands::create_vault(deps, env, info, asset_info, fees, vault_identifier),
        ExecuteMsg::UpdateConfig {
            whale_lair_addr,
            vault_creation_fee,
            cw20_lp_code_id,
            flash_loan_enabled,
            deposit_enabled,
            withdraw_enabled,
        } => manager::commands::update_config(
            deps,
            info,
            whale_lair_addr,
            vault_creation_fee,
            cw20_lp_code_id,
            flash_loan_enabled,
            deposit_enabled,
            withdraw_enabled,
        ),
        ExecuteMsg::Deposit {
            asset,
            vault_identifier,
        } => vault::commands::deposit(deps, &env, &info, &asset, &vault_identifier),
        ExecuteMsg::Withdraw => {
            let lp_asset = AssetInfo::NativeToken {
                denom: one_coin(&info)?.denom,
            };

            // check if the vault exists and the asset matches
            let vault = get_vault_by_lp(&deps.as_ref(), &lp_asset)?;
            assert_asset(&vault.lp_asset, &lp_asset)?;

            vault::commands::withdraw(
                deps,
                env,
                info.sender.into_string(),
                info.funds[0].amount,
                vault,
            )
        }
        ExecuteMsg::Receive(msg) => {
            // check if it's a cw20 lp asset executing this callback
            let lp_asset = AssetInfo::Token {
                contract_addr: info.sender.into_string(),
            };

            let vault = get_vault_by_lp(&deps.as_ref(), &lp_asset)?;
            assert_asset(&vault.lp_asset, &lp_asset)?;

            match from_json(&msg.msg)? {
                Cw20HookMsg::Withdraw => {
                    vault::commands::withdraw(deps, env, msg.sender, msg.amount, vault)
                }
            }
        }
        ExecuteMsg::FlashLoan {
            asset,
            vault_identifier,
            payload,
        } => router::commands::flash_loan(deps, env, info, asset, vault_identifier, payload),
        ExecuteMsg::Callback(msg) => router::commands::callback(deps, env, info, msg),
        ExecuteMsg::UpdateOwnership(action) => {
            Ok(
                cw_ownable::update_ownership(deps, &env.block, &info.sender, action).map(
                    |ownership| {
                        Response::default()
                            .add_attribute("action", "update_ownership")
                            .add_attributes(ownership.into_attributes())
                    },
                )?,
            )
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
