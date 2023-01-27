use cosmwasm_std::{
    Addr, BankMsg, Coin, CosmosMsg, DepsMut, MessageInfo, Order, Response, StdError, StdResult,
    Uint128,
};

use crate::queries::MAX_CLAIM_LIMIT;
use crate::state::{update_global_weight, update_local_weight, CONFIG, GLOBAL, STAKE, UNSTAKE};
use crate::ContractError;

/// Stakes the provided amount of tokens.
pub(crate) fn stake(
    mut deps: DepsMut,
    block_height: u64,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // validate the denom sent is the whitelisted one for staking
    let denom = CONFIG.load(deps.storage)?.staking_denom;
    if info.funds.len() != 1 || info.funds[0].denom != denom || info.funds[0].amount != amount {
        return Err(ContractError::AssetMismatch {});
    }

    let mut stake = STAKE
        .may_load(deps.storage, &info.sender)?
        .unwrap_or_default();

    // update local values
    stake = update_local_weight(&mut deps, info.sender.clone(), block_height, stake)?;
    stake.amount += amount;
    STAKE.save(deps.storage, &info.sender, &stake)?;

    // update global values
    let mut global_index = GLOBAL.may_load(deps.storage)?.unwrap_or_default();
    global_index = update_global_weight(&mut deps, block_height, global_index)?;
    global_index.stake += amount;
    GLOBAL.save(deps.storage, &global_index)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "stake".to_string()),
        ("address", info.sender.to_string()),
        ("amount", amount.to_string()),
    ]))
}

/// Unstakes the provided amount of tokens
pub(crate) fn unstake(
    mut deps: DepsMut,
    block_height: u64,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // check if the address has enough stake
    let mut stake = STAKE
        .may_load(deps.storage, &info.sender)?
        .unwrap_or_default();
    if stake.amount < amount {
        return Err(ContractError::InsufficientStake {});
    }

    // update local values, decrease the stake
    stake = update_local_weight(&mut deps, info.sender.clone(), block_height, stake.clone())?;
    let weight_slash = stake
        .weight
        .checked_mul(amount.checked_div(stake.amount)?)?;
    stake.amount -= amount;
    stake.weight -= weight_slash;
    STAKE.save(deps.storage, &info.sender, &stake)?;

    // record the unstaking
    UNSTAKE.save(
        deps.storage,
        (info.sender.as_bytes(), block_height),
        &white_whale::whale_lair::Stake {
            amount,
            weight: Uint128::zero(),
            block_height,
        },
    )?;

    // update global values
    let mut global_index = GLOBAL.may_load(deps.storage)?.unwrap_or_default();
    global_index = update_global_weight(&mut deps, block_height, global_index)?;
    global_index.stake -= amount;
    global_index.weight -= weight_slash;
    GLOBAL.save(deps.storage, &global_index)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "unstake".to_string()),
        ("address", info.sender.to_string()),
        ("amount", amount.to_string()),
    ]))
}

/// Claims the rewards for the provided address
pub(crate) fn claim(
    deps: DepsMut,
    block_height: u64,
    address: Addr,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let unstakes: StdResult<Vec<_>> = UNSTAKE
        .prefix(address.as_bytes())
        .range(deps.storage, None, None, Order::Ascending)
        .take(MAX_CLAIM_LIMIT as usize)
        .collect();

    let mut refund_amount = Uint128::zero();
    for item in unstakes? {
        let (block, stake) = item;
        if block_height
            >= stake
                .block_height
                .checked_add(config.unstaking_period)
                .ok_or_else(|| StdError::generic_err("Invalid block height"))?
        {
            refund_amount += stake.amount;
            UNSTAKE.remove(deps.storage, (address.as_bytes(), block));
        }
    }

    let refund_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: address.to_string(),
        amount: vec![Coin {
            denom: config.staking_denom,
            amount: refund_amount,
        }],
    });

    Ok(Response::new().add_message(refund_msg).add_attributes(vec![
        ("action", "claim".to_string()),
        ("address", address.to_string()),
    ]))
}

/// Updates the configuration of the contract
pub(crate) fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<String>,
    unstaking_period: Option<u64>,
    growth_rate: Option<u8>,
) -> Result<Response, ContractError> {
    // check the owner is the one who sent the message
    let mut config = CONFIG.load(deps.storage)?;
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(owner) = owner {
        config.owner = deps.api.addr_validate(&owner)?;
    }

    if let Some(unstaking_period) = unstaking_period {
        config.unstaking_period = unstaking_period;
    }

    if let Some(growth_rate) = growth_rate {
        config.growth_rate = growth_rate;
    }

    Ok(Response::new().add_attributes(vec![
        ("action", "update_config".to_string()),
        ("owner", config.owner.to_string()),
        ("unstaking_period", config.unstaking_period.to_string()),
        ("growth_rate", config.growth_rate.to_string()),
    ]))
}
