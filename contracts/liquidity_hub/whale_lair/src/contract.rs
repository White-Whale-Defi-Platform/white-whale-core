use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;

use white_whale::validators::validate_denom;
use white_whale::whale_lair::{Config, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

use crate::error::ContractError;
use crate::state::CONFIG;
use crate::ContractError::MigrateInvalidVersion;
use crate::{commands, queries};

// version info for migration info
const CONTRACT_NAME: &str = "white_whale-whale_lair";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    validate_denom(deps.as_ref(), &env, &msg.bonding_denom, &info.funds)?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: deps.api.addr_validate(info.sender.as_str())?,
        unbonding_period: msg.unbonding_period,
        growth_rate: msg.growth_rate,
        bonding_denom: msg.bonding_denom,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default().add_attributes(vec![
        ("action", "instantiate".to_string()),
        ("owner", config.owner.to_string()),
        ("unbonding_period", config.unbonding_period.to_string()),
        ("growth_rate", config.growth_rate.to_string()),
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
        ExecuteMsg::Bond { amount } => commands::bond(deps, env.block.height, info, amount),
        ExecuteMsg::Unbond { amount } => commands::unbond(deps, env.block.height, info, amount),
        ExecuteMsg::Withdraw {} => commands::withdraw(deps, env.block.height, info.sender),
        ExecuteMsg::UpdateConfig {
            owner,
            unbonding_period,
            growth_rate,
        } => commands::update_config(deps, info, owner, unbonding_period, growth_rate),
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&queries::query_config(deps)?),
        QueryMsg::Bonded { address } => to_binary(&queries::query_bonded(deps, address)?),
        QueryMsg::Unbonding {
            address,
            start_after,
            limit,
        } => to_binary(&queries::query_unbonding(
            deps,
            address,
            start_after,
            limit,
        )?),
        QueryMsg::Withdrawable { address } => to_binary(&queries::query_withdrawable(
            deps,
            env.block.height,
            address,
        )?),
        QueryMsg::Weight { address } => {
            to_binary(&queries::query_weight(deps, env.block.height, address)?)
        }
    }
}

#[cfg(not(tarpaulin_include))]
#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
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
