use cosmwasm_std::{Decimal, DepsMut, MessageInfo, Response};

use crate::helpers::validate_growth_rate;
use crate::state::CONFIG;
use crate::ContractError;

/// Updates the configuration of the contract
pub(crate) fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    epoch_manager_addr: Option<String>,
    pool_manager_addr: Option<String>,
    unbonding_period: Option<u64>,
    growth_rate: Option<Decimal>,
) -> Result<Response, ContractError> {
    // check the owner is the one who sent the message
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut config = CONFIG.load(deps.storage)?;

    if let Some(epoch_manager_addr) = epoch_manager_addr {
        config.epoch_manager_addr = deps.api.addr_validate(&epoch_manager_addr)?;
    }

    if let Some(pool_manager_addr) = pool_manager_addr {
        config.pool_manager_addr = deps.api.addr_validate(&pool_manager_addr)?;
    }

    if let Some(unbonding_period) = unbonding_period {
        config.unbonding_period = unbonding_period;
    }

    if let Some(growth_rate) = growth_rate {
        validate_growth_rate(growth_rate)?;
        config.growth_rate = growth_rate;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default().add_attributes(vec![
        ("action", "update_config".to_string()),
        ("pool_manager_addr", config.pool_manager_addr.to_string()),
        ("epoch_manager_addr", config.epoch_manager_addr.to_string()),
        ("unbonding_period", config.unbonding_period.to_string()),
        ("growth_rate", config.growth_rate.to_string()),
    ]))
}
