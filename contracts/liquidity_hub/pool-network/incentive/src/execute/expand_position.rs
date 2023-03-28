use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdError, Uint128};

use crate::{
    error::ContractError,
    state::{ADDRESS_WEIGHT, GLOBAL_WEIGHT, OPEN_POSITIONS},
    weight::calculate_weight,
};

pub fn expand_position(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
    unbonding_duration: u64,
) -> Result<Response, ContractError> {
    // claim current position
    let claim_messages = crate::claim::claim(&mut deps, &env, &info)?;

    // increase position
    OPEN_POSITIONS.update::<_, ContractError>(deps.storage, info.sender.clone(), |positions| {
        let mut positions =
            positions.ok_or(ContractError::NonExistentPosition { unbonding_duration })?;

        let pos = positions
            .iter_mut()
            .find(|position| position.unbonding_duration == unbonding_duration)
            .ok_or(ContractError::NonExistentPosition { unbonding_duration })?;

        pos.amount += amount;

        Ok(positions)
    })?;

    // add the weight
    let weight = calculate_weight(unbonding_duration, amount)?;
    GLOBAL_WEIGHT.update::<_, StdError>(deps.storage, |global_weight| {
        Ok(global_weight.checked_add(weight)?)
    })?;
    ADDRESS_WEIGHT.update::<_, StdError>(deps.storage, info.sender, |user_weight| {
        Ok(user_weight.unwrap_or_default().checked_add(weight)?)
    })?;

    Ok(Response::new().add_messages(claim_messages))
}
