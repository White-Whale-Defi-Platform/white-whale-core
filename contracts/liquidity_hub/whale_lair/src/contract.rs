use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, to_binary};
use cosmwasm_std::entry_point;
use cw2::set_contract_version;

use white_whale::whale_lair::{Config, ExecuteMsg, InstantiateMsg, QueryMsg};

use crate::error::ContractError;
use crate::queries;
use crate::state::CONFIG;

// version info for migration info
const CONTRACT_NAME: &str = "white_whale-whale_lair";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: deps.api.addr_validate(info.sender.as_str())?,
        unstaking_period: msg.unstaking_period,
        growth_rate: msg.growth_rate,
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
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Stake { amount } => unimplemented!(),
        ExecuteMsg::Unstake { amount } => unimplemented!(),
        ExecuteMsg::Claim {} => unimplemented!(),
        ExecuteMsg::UpdateConfig { owner, unstaking_period, growth_rate } => unimplemented!(),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&queries::query_config(deps)?),
        QueryMsg::Staked { address } => unimplemented!(),
        QueryMsg::Unstaking { address } => unimplemented!(),
        QueryMsg::Claimable { address } => unimplemented!(),
        QueryMsg::Weight { address } => unimplemented!(),
    }
}
