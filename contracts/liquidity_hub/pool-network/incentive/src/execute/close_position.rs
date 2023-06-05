use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdError};

use white_whale::pool_network::incentive::{ClosedPosition, OpenPosition};

use crate::queries::get_rewards;
use crate::state::ADDRESS_WEIGHT_HISTORY;
use crate::{
    error::ContractError,
    helpers,
    state::{ADDRESS_WEIGHT, CLOSED_POSITIONS, GLOBAL_WEIGHT, OPEN_POSITIONS},
    weight::calculate_weight,
};

/// Closes the position for the user with the given unbonding_duration.
pub fn close_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    unbonding_duration: u64,
) -> Result<Response, ContractError> {
    //query and check if the user has pending rewards
    let rewards_query_result = get_rewards(deps.as_ref(), info.sender.clone().into_string());
    match rewards_query_result {
        Ok(rewards_response) => {
            // can't close a position if there are pending rewards
            if !rewards_response.rewards.is_empty() {
                return Err(ContractError::PendingRewards {});
            }
        }
        Err(error) => {
            //if it has nothing to claim, then close the position
            match error {
                ContractError::NothingToClaim {} => {}
                _ => return Err(ContractError::InvalidReward {}),
            }
        }
    }

    // remove position
    let mut open_positions = OPEN_POSITIONS
        .may_load(deps.storage, info.sender.clone())?
        .ok_or(ContractError::NonExistentPosition { unbonding_duration })?;
    let to_close_index = open_positions
        .iter()
        .position(|pos| pos.unbonding_duration == unbonding_duration)
        .ok_or(ContractError::NonExistentPosition { unbonding_duration })?;
    let to_close_position = &open_positions[to_close_index];

    // move to a closed position
    CLOSED_POSITIONS.update::<_, ContractError>(
        deps.storage,
        info.sender.clone(),
        |closed_positions| {
            let mut closed_positions = closed_positions.unwrap_or_default();

            closed_positions.push(ClosedPosition {
                amount: to_close_position.amount,
                unbonding_timestamp: env
                    .block
                    .time
                    .seconds()
                    .checked_add(to_close_position.unbonding_duration)
                    .ok_or(ContractError::OverflowTimestamp)?,
            });

            Ok(closed_positions)
        },
    )?;

    // reduce weight
    // we reduce the weight to be equivalent to 1*amount, so we subtract by (weight - amount)
    // this should always be a valid operation as calculate_weight will return >= amount
    let weight_to_reduce = calculate_weight(unbonding_duration, to_close_position.amount)?;

    // reduce the global weight
    GLOBAL_WEIGHT.update::<_, StdError>(deps.storage, |global_weight| {
        Ok(global_weight.checked_sub(weight_to_reduce)?)
    })?;

    // reduce the weight for the user
    let mut user_weight = ADDRESS_WEIGHT
        .may_load(deps.storage, info.sender.clone())?
        .unwrap_or_default();
    user_weight = user_weight.checked_sub(weight_to_reduce)?;
    ADDRESS_WEIGHT.save(deps.storage, info.sender.clone(), &user_weight)?;

    let current_epoch = helpers::get_current_epoch(deps.as_ref())?;

    // store new user weight in history for the next epoch
    ADDRESS_WEIGHT_HISTORY.update::<_, StdError>(
        deps.storage,
        (&info.sender, current_epoch + 1u64),
        |_| Ok(user_weight),
    )?;

    // remove closed position from open positions map
    let closing_position: OpenPosition = open_positions[to_close_index].clone();

    open_positions.remove(to_close_index);
    OPEN_POSITIONS.save(deps.storage, info.sender, &open_positions)?;

    Ok(Response::default().add_attributes(vec![
        ("action", "close_position".to_string()),
        ("closing_position", closing_position.to_string()),
        ("unbonding_duration", unbonding_duration.to_string()),
    ]))
}
