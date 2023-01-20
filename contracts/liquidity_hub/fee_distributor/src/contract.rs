use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, to_binary};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cw2::set_contract_version;

use crate::{commands, queries, state};
use crate::error::ContractError;
use crate::helpers::validate_grace_period;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, CONFIG};

// version info for migration info
const CONTRACT_NAME: &str = "white_whale-fee_distributor";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    validate_grace_period(&msg.grace_period)?;

    let config = Config {
        owner: deps.api.addr_validate(info.sender.as_str())?,
        staking_contract_addr: deps.api.addr_validate(msg.staking_contract_addr.as_str())?,
        fee_collector_addr: deps.api.addr_validate(msg.fee_collector_addr.as_str())?,
        grace_period: msg.grace_period,
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
        .add_attribute("grace_period", config.grace_period.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::NewEpoch { fees } => commands::create_new_epoch(deps, info, fees),
        ExecuteMsg::Claim {} => commands::claim(deps, info),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::CurrentEpoch {} => Ok(to_binary(&state::get_current_epoch(deps)?)?),
        QueryMsg::Epoch { id } => Ok(to_binary(&state::get_epoch(deps, id)?)?),
        QueryMsg::ClaimableEpochs {} => Ok(to_binary(&state::get_claimable_epochs(deps)?)?),
        QueryMsg::Config {} => Ok(to_binary(&queries::query_config(deps)?)?),
    }
}
