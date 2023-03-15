#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult,
};
use cw2::set_contract_version;
use white_whale::fee_distributor::{Config, ExecuteMsg, InstantiateMsg, QueryMsg};

use crate::error::ContractError;
use crate::helpers::{validate_epoch_config, validate_grace_period};
use crate::state::{get_expiring_epoch, CONFIG, EPOCHS};
use crate::{commands, helpers, queries, state};

// version info for migration info
const CONTRACT_NAME: &str = "white_whale-fee_distributor";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub(crate) const EPOCH_CREATION_REPLY_ID: u64 = 1;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    validate_grace_period(&msg.grace_period)?;
    validate_epoch_config(&msg.epoch_config)?;

    let config = Config {
        owner: deps.api.addr_validate(info.sender.as_str())?,
        staking_contract_addr: deps.api.addr_validate(msg.bonding_contract_addr.as_str())?,
        fee_collector_addr: deps.api.addr_validate(msg.fee_collector_addr.as_str())?,
        grace_period: msg.grace_period,
        epoch_config: msg.epoch_config,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("owner", config.owner.as_str())
        .add_attribute(
            "staking_contract_addr",
            config.staking_contract_addr.as_str(),
        )
        .add_attribute("fee_collector_addr", config.fee_collector_addr.as_str())
        .add_attribute("grace_period", config.grace_period.to_string())
        .add_attribute("epoch_config", config.epoch_config.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
    // forward fees from previous epoch to the new one

    // read temp epoch from binary data and fill in the vectors with the sent tokens

    //EPOCHS.save(deps.storage, &new_epoch.id.to_be_bytes(), &new_epoch)?;
    Ok(Response::new())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::NewEpoch {} => commands::create_new_epoch(deps, info, env),
        ExecuteMsg::Claim {} => commands::claim(deps, info),
        ExecuteMsg::UpdateConfig {
            owner,
            staking_contract_addr,
            fee_collector_addr,
            grace_period,
        } => commands::update_config(
            deps,
            info,
            env,
            owner,
            staking_contract_addr,
            fee_collector_addr,
            grace_period,
        ),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::CurrentEpoch {} => Ok(to_binary(&state::get_current_epoch(deps)?)?),
        QueryMsg::Epoch { id } => Ok(to_binary(&state::get_epoch(deps, id)?)?),
        QueryMsg::ClaimableEpochs {} => Ok(to_binary(&state::get_claimable_epochs(deps)?)?),
        QueryMsg::Config {} => Ok(to_binary(&queries::query_config(deps)?)?),
    }
}
