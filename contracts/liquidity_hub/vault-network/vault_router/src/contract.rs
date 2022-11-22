use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;
use vault_network::vault_router::{Config, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

use crate::err::{StdResult, VaultRouterError};
use crate::execute::{complete_loan, flash_loan, next_loan, update_config};
use crate::queries::get_config;
use crate::state::CONFIG;

const CONTRACT_NAME: &str = "white_whale-vault_router";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: deps.api.addr_validate(&msg.owner)?,
        vault_factory: deps.api.addr_validate(&msg.vault_factory_addr)?,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::FlashLoan { assets, msgs } => flash_loan(deps, info, assets, msgs),
        ExecuteMsg::NextLoan {
            initiator,
            source_vault,
            source_vault_asset_info: source_vault_asset,
            payload,
            to_loan,
            loaned_assets,
        } => next_loan(
            deps,
            env,
            info,
            payload,
            initiator,
            source_vault,
            source_vault_asset,
            to_loan,
            loaned_assets,
        ),
        ExecuteMsg::CompleteLoan {
            initiator,
            loaned_assets,
        } => complete_loan(deps, env, info, initiator, loaned_assets),
        ExecuteMsg::UpdateConfig {
            owner,
            vault_factory_addr,
        } => update_config(deps, info, owner, vault_factory_addr),
    }
}

#[cfg(not(tarpaulin_include))]
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    let version: Version = CONTRACT_VERSION.parse()?;
    let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;

    if storage_version >= version {
        return Err(VaultRouterError::MigrateInvalidVersion {
            current_version: storage_version,
            new_version: version,
        });
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => get_config(deps),
    }
}
