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
    validate_denom(deps.as_ref(), &env, &msg.staking_denom, &info.funds)?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: deps.api.addr_validate(info.sender.as_str())?,
        unstaking_period: msg.unstaking_period,
        growth_rate: msg.growth_rate,
        staking_denom: msg.staking_denom,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default().add_attributes(vec![
        ("action", "instantiate".to_string()),
        ("owner", config.owner.to_string()),
        ("unstaking_period", config.unstaking_period.to_string()),
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
        ExecuteMsg::Stake { amount } => commands::stake(deps, env.block.height, info, amount),
        ExecuteMsg::Unstake { amount } => commands::unstake(deps, env.block.height, info, amount),
        ExecuteMsg::Claim {} => commands::claim(deps, env.block.height, info.sender),
        ExecuteMsg::UpdateConfig {
            owner,
            unstaking_period,
            growth_rate,
        } => commands::update_config(deps, info, owner, unstaking_period, growth_rate),
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&queries::query_config(deps)?),
        QueryMsg::Staked { address } => to_binary(&queries::query_staked(deps, address)?),
        QueryMsg::Unstaking {
            address,
            start_after,
            limit,
        } => to_binary(&queries::query_unstaking(
            deps,
            address,
            start_after,
            limit,
        )?),
        QueryMsg::Claimable { address } => {
            to_binary(&queries::query_claimable(deps, env.block.height, address)?)
        }
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
