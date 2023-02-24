use cosmwasm_std::{
    Addr, BankMsg, Coin, CosmosMsg, DepsMut, MessageInfo, Order, Response, StdError, StdResult,
    Uint128,
};

use crate::queries::MAX_CLAIM_LIMIT;
use crate::state::{update_global_weight, update_local_weight, BOND, CONFIG, GLOBAL, UNBOND};
use crate::ContractError;

/// Bonds the provided amount of tokens.
pub(crate) fn bond(
    mut deps: DepsMut,
    block_height: u64,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // validate the denom sent is the whitelisted one for bonding
    let denom = CONFIG.load(deps.storage)?.bonding_denom;
    if info.funds.len() != 1 || info.funds[0].denom != denom || info.funds[0].amount != amount {
        return Err(ContractError::AssetMismatch {});
    }

    let mut bond = BOND
        .may_load(deps.storage, &info.sender)?
        .unwrap_or_default();

    // update local values
    bond = update_local_weight(&mut deps, info.sender.clone(), block_height, bond)?;
    bond.amount = bond.amount.checked_add(amount)?;
    BOND.save(deps.storage, &info.sender, &bond)?;

    // update global values
    let mut global_index = GLOBAL.may_load(deps.storage)?.unwrap_or_default();
    global_index = update_global_weight(&mut deps, block_height, global_index)?;
    global_index.bond = global_index.bond.checked_add(amount)?;
    GLOBAL.save(deps.storage, &global_index)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "bond".to_string()),
        ("address", info.sender.to_string()),
        ("amount", amount.to_string()),
    ]))
}

/// Unbonds the provided amount of tokens
pub(crate) fn unbond(
    mut deps: DepsMut,
    block_height: u64,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // check if the address has enough bond
    let mut unbond = BOND
        .may_load(deps.storage, &info.sender)?
        .unwrap_or_default();
    if unbond.amount < amount {
        return Err(ContractError::InsufficientBond {});
    }

    // update local values, decrease the bond
    unbond = update_local_weight(&mut deps, info.sender.clone(), block_height, unbond.clone())?;
    let weight_slash = unbond
        .weight
        .checked_mul(amount.checked_div(unbond.amount)?)?;
    unbond.amount = unbond.amount.checked_sub(amount)?;
    unbond.weight = unbond.weight.checked_sub(weight_slash)?;
    BOND.save(deps.storage, &info.sender, &unbond)?;

    // record the unbonding
    UNBOND.save(
        deps.storage,
        (&info.sender, block_height),
        &white_whale::whale_lair::Bond {
            amount,
            weight: Uint128::zero(),
            block_height,
        },
    )?;

    // update global values
    let mut global_index = GLOBAL.may_load(deps.storage)?.unwrap_or_default();
    global_index = update_global_weight(&mut deps, block_height, global_index)?;
    global_index.bond = global_index.bond.checked_sub(amount)?;
    global_index.weight = global_index.weight.checked_sub(weight_slash)?;
    GLOBAL.save(deps.storage, &global_index)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "unbond".to_string()),
        ("address", info.sender.to_string()),
        ("amount", amount.to_string()),
    ]))
}

/// Withdraws the rewards for the provided address
pub(crate) fn withdraw(
    deps: DepsMut,
    block_height: u64,
    address: Addr,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let unbondings: StdResult<Vec<_>> = UNBOND
        .prefix(&address)
        .range(deps.storage, None, None, Order::Ascending)
        .take(MAX_CLAIM_LIMIT as usize)
        .collect();

    let mut refund_amount = Uint128::zero();
    for unbonding in unbondings? {
        let (block, bond) = unbonding;
        if block_height
            >= bond
                .block_height
                .checked_add(config.unbonding_period)
                .ok_or_else(|| StdError::generic_err("Invalid block height"))?
        {
            refund_amount = refund_amount.checked_add(bond.amount)?;
            UNBOND.remove(deps.storage, (&address, block));
        }
    }

    let refund_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: address.to_string(),
        amount: vec![Coin {
            denom: config.bonding_denom,
            amount: refund_amount,
        }],
    });

    Ok(Response::new().add_message(refund_msg).add_attributes(vec![
        ("action", "withdraw".to_string()),
        ("address", address.to_string()),
    ]))
}

/// Updates the configuration of the contract
pub(crate) fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<String>,
    unbonding_period: Option<u64>,
    growth_rate: Option<u8>,
    bonding_denom: Option<String>,
) -> Result<Response, ContractError> {
    // check the owner is the one who sent the message
    let mut config = CONFIG.load(deps.storage)?;
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(owner) = owner {
        config.owner = deps.api.addr_validate(&owner)?;
    }

    if let Some(unbonding_period) = unbonding_period {
        config.unbonding_period = unbonding_period;
    }

    if let Some(growth_rate) = growth_rate {
        config.growth_rate = growth_rate;
    }

    if let Some(bonding_denom) = bonding_denom {
        config.bonding_denom = bonding_denom;
    }

    Ok(Response::new().add_attributes(vec![
        ("action", "update_config".to_string()),
        ("owner", config.owner.to_string()),
        ("unbonding_period", config.unbonding_period.to_string()),
        ("growth_rate", config.growth_rate.to_string()),
    ]))
}
