use cosmwasm_std::{CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError};

use white_whale::incentive_manager::{Position, PositionParams};

use crate::helpers::validate_unbonding_duration;
use crate::position::helpers::{calculate_weight, validate_funds_sent};
use crate::state::{
    ADDRESS_LP_WEIGHT, ADDRESS_LP_WEIGHT_HISTORY, CONFIG, LP_WEIGHTS, OPEN_POSITIONS,
};
use crate::ContractError;

pub(crate) fn fill_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    params: PositionParams,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // validate unbonding duration
    validate_unbonding_duration(&config, &params)?;

    let mut messages: Vec<CosmosMsg> = vec![];

    // ensure the lp tokens are transferred to the contract. If the LP is a cw20 token, creates
    // a transfer message
    let transfer_token_msg = validate_funds_sent(
        &deps.as_ref(),
        env.clone(),
        info.clone(),
        params.clone().lp_asset,
    )?;

    if let Some(transfer_token_msg) = transfer_token_msg {
        messages.push(transfer_token_msg.into());
    }

    // if receiver was not specified, default to the sender of the message.
    let receiver = params
        .clone()
        .receiver
        .map(|r| deps.api.addr_validate(&r))
        .transpose()?
        .map(|receiver| MessageInfo {
            funds: info.funds.clone(),
            sender: receiver,
        })
        .unwrap_or_else(|| info.clone());

    // check if there's an existing position with the given `unbonding_time`, get the index of it
    // on the vector
    let position_index = OPEN_POSITIONS
        .may_load(deps.storage, &receiver.sender)?
        .unwrap_or_default()
        .iter()
        .enumerate()
        .find(|(_, position)| position.unbonding_duration == params.unbonding_duration)
        .map(|(index, _)| index);

    // update the position
    OPEN_POSITIONS.update::<_, ContractError>(deps.storage, &receiver.sender, |positions| {
        let mut positions = positions.unwrap_or_default();

        // if the position exists, expand it. Otherwise, create a new position by adding it to the vector
        match position_index {
            Some(index) => {
                // Update the existing position at the given index
                if let Some(pos) = positions.get_mut(index) {
                    if pos.lp_asset.info != params.lp_asset.info {
                        return Err(ContractError::AssetMismatch);
                    }
                    pos.lp_asset.amount =
                        pos.lp_asset.amount.checked_add(params.lp_asset.amount)?;
                }
            }
            None => {
                positions.push(Position {
                    lp_asset: params.clone().lp_asset,
                    unbonding_duration: params.unbonding_duration,
                });
            }
        }

        Ok(positions)
    })?;

    // Update weights for the LP and the user
    update_weights(deps, &receiver, &params)?;

    let action = match position_index {
        Some(_) => "expand_position",
        None => "open_position",
    };

    Ok(Response::default().add_attributes(vec![
        ("action", action.to_string()),
        ("receiver", receiver.sender.to_string()),
        ("params", params.clone().to_string()),
    ]))
}

/// Updates the weights when managing a position
fn update_weights(
    deps: DepsMut,
    receiver: &MessageInfo,
    params: &PositionParams,
) -> Result<(), ContractError> {
    let weight = calculate_weight(params)?;
    LP_WEIGHTS.update::<_, StdError>(
        deps.storage,
        &params.lp_asset.info.as_bytes(),
        |lp_weight| Ok(lp_weight.unwrap_or_default().checked_add(weight)?),
    )?;

    // update the user's weight for this LP
    let mut address_lp_weight = ADDRESS_LP_WEIGHT
        .may_load(
            deps.storage,
            (&receiver.sender, &params.lp_asset.info.as_bytes()),
        )?
        .unwrap_or_default();
    address_lp_weight = address_lp_weight.checked_add(weight)?;
    ADDRESS_LP_WEIGHT.save(
        deps.storage,
        (&receiver.sender, &params.lp_asset.info.as_bytes()),
        &address_lp_weight,
    )?;

    let config = CONFIG.load(deps.storage)?;
    let current_epoch = white_whale::epoch_manager::common::get_current_epoch(
        deps.as_ref(),
        config.epoch_manager_addr.clone().into_string(),
    )?;

    ADDRESS_LP_WEIGHT_HISTORY.update::<_, StdError>(
        deps.storage,
        (&receiver.sender, current_epoch + 1u64),
        |_| Ok(address_lp_weight),
    )?;

    Ok(())
}

/// Closes an existing position
pub(crate) fn close_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    unbonding_duration: u64,
) -> Result<Response, ContractError> {
    Ok(Response::default().add_attributes(vec![("action", "close_position".to_string())]))
}
