use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdError};

use white_whale::pool_network::incentive::{ClosedPosition, OpenPosition, RewardsResponse};

use crate::claim::claim2;
use crate::queries::get_rewards;
use crate::state::{ADDRESS_WEIGHT_HISTORY, CONFIG};
use crate::{
    error::ContractError,
    state::{ADDRESS_WEIGHT, CLOSED_POSITIONS, GLOBAL_WEIGHT, OPEN_POSITIONS},
    weight::calculate_weight,
};

pub fn close_position(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    unbonding_duration: u64,
) -> Result<Response, ContractError> {
    //query and see if the user has pending rewards
    let rewards_query_result = get_rewards(
        deps.as_ref(),
        env.clone(),
        info.sender.clone().into_string(),
    );
    match rewards_query_result {
        Ok(rewards_response) => {
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

    // todo remove
    // claim current position
    //let claim_messages = crate::claim::claim2(&mut deps, &env, &info)?;

    // remove position
    let mut open_positions = OPEN_POSITIONS
        .may_load(deps.storage, info.sender.clone())?
        .ok_or(ContractError::NonExistentPosition { unbonding_duration })?;
    let to_close_index = open_positions
        .iter()
        .position(|pos| pos.unbonding_duration == unbonding_duration)
        .ok_or(ContractError::NonExistentPosition { unbonding_duration })?;
    let to_close_position = &open_positions[to_close_index];

    println!("to_close_position: {:?}", to_close_position);

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
    //let reduced_weight = weight.checked_sub(to_close_position.amount)?;

    println!("weight_to_reduce: {:?}", weight_to_reduce);

    GLOBAL_WEIGHT.update::<_, StdError>(deps.storage, |global_weight| {
        println!("global weight before closing position: {:?}", global_weight);
        println!(
            "global weight after closing position: {:?}",
            global_weight.checked_sub(weight_to_reduce).unwrap()
        );

        Ok(global_weight.checked_sub(weight_to_reduce)?)
    })?;
    // ADDRESS_WEIGHT.update::<_, StdError>(deps.storage, info.sender.clone(), |user_weight| {
    //     Ok(user_weight
    //         .unwrap_or_default()
    //         .checked_sub(reduced_weight)?)
    // })?;

    // TODO new stuff, remove/refactor old stuff

    let config = CONFIG.load(deps.storage)?;

    let epoch_response: white_whale::fee_distributor::EpochResponse =
        deps.querier.query_wasm_smart(
            config.fee_distributor_address.into_string(),
            &white_whale::fee_distributor::QueryMsg::CurrentEpoch {},
        )?;

    let mut user_weight = ADDRESS_WEIGHT
        .may_load(deps.storage, info.sender.clone())?
        .unwrap_or_default();

    println!("user_weight: {}", user_weight);

    user_weight = user_weight.checked_sub(weight_to_reduce)?;

    println!("user_weight after closing position: {}", user_weight);

    ADDRESS_WEIGHT_HISTORY.update::<_, StdError>(
        deps.storage,
        (&info.sender.clone(), epoch_response.epoch.id.u64() + 1u64),
        |_| Ok(user_weight),
    )?;

    ADDRESS_WEIGHT.save(deps.storage, info.sender.clone(), &user_weight)?;

    // ADDRESS_WEIGHT.update::<_, StdError>(deps.storage, info.sender.clone(), |user_weight| {
    //     let new_user_weight = user_weight
    //         .unwrap_or_default()
    //         .checked_sub(reduced_weight)?;
    //
    //     ADDRESS_WEIGHT_HISTORY.update::<_, StdError>(
    //         deps.storage,
    //         (&info.sender.clone(), epoch_response.epoch.id.u64()),
    //         |user_weight| Ok(new_user_weight),
    //     )?;
    //
    //     Ok(new_user_weight)
    // })?;

    let closing_position: OpenPosition = open_positions[to_close_index].clone();

    open_positions.remove(to_close_index);
    OPEN_POSITIONS.save(deps.storage, info.sender, &open_positions)?;

    Ok(Response::default().add_attributes(vec![
        ("action", "close_position".to_string()),
        ("closing_position", closing_position.to_string()),
        ("unbonding_duration", unbonding_duration.to_string()),
    ]))
}
