use cosmwasm_std::{entry_point, from_binary};
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError};
use cw2::{get_contract_version, set_contract_version};
use cw_utils::one_coin;
use semver::Version;

use white_whale::pool_network::asset::AssetInfo;
use white_whale::vault_manager::{
    Config, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};

use crate::error::ContractError;
use crate::state::{get_vault, CONFIG, ONGOING_FLASHLOAN};
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
        ExecuteMsg::CreateVault { asset_info, fees } => {
            manager::commands::create_vault(deps, env, info, asset_info, fees)
        }
        ExecuteMsg::RemoveVault { asset_info } => {
            manager::commands::remove_vault(deps, info, asset_info)
        }
        ExecuteMsg::UpdateVaultFees {
            vault_asset_info,
            vault_fee,
        } => manager::commands::update_vault_fees(deps, info, vault_asset_info, vault_fee),
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
        ExecuteMsg::Deposit { asset } => vault::commands::deposit(&deps, &env, &info, &asset),
        ExecuteMsg::Withdraw {} => {
            let lp_asset = AssetInfo::NativeToken {
                denom: one_coin(&info)?.denom,
            };

            // check if the vault exists
            let vault = get_vault(&deps.as_ref(), &lp_asset)?;

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

            let vault = get_vault(&deps.as_ref(), &lp_asset)?;

            match from_binary(&msg.msg)? {
                Cw20HookMsg::Withdraw {} => {
                    vault::commands::withdraw(deps, env, msg.sender, msg.amount, vault)
                }
            }
        }
        ExecuteMsg::FlashLoan { asset, payload } => {
            router::commands::flash_loan(deps, env, info, asset, payload)
        }
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
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Config {} => Ok(to_binary(&queries::query_manager_config(deps)?)?),
        QueryMsg::Vault { asset_info } => Ok(to_binary(&queries::query_vault(deps, asset_info)?)?),
        QueryMsg::Vaults { start_after, limit } => Ok(to_binary(&queries::query_vaults(
            deps,
            start_after,
            limit,
        )?)?),
        QueryMsg::Share { lp_share } => Ok(to_binary(&queries::get_share(deps, env, lp_share)?)?),
        QueryMsg::PaybackAmount { asset } => {
            Ok(to_binary(&queries::get_payback_amount(deps, asset)?)?)
        }
        QueryMsg::Ownership {} => Ok(to_binary(&cw_ownable::get_ownership(deps.storage)?)?),
    }
}

#[cfg(not(tarpaulin_include))]
#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    use white_whale::migrate_guards::check_contract_name;

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
