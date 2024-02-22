use cosmwasm_std::{CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError};

use white_whale::incentive_manager::Position;
use white_whale::pool_network::asset::Asset;

use crate::helpers::validate_unlocking_duration;
use crate::position::helpers::{
    calculate_weight, get_latest_address_weight, get_latest_lp_weight, validate_funds_sent,
};
use crate::state::{
    get_position, ADDRESS_LP_WEIGHT_HISTORY, CONFIG, LP_WEIGHTS_HISTORY, POSITIONS,
    POSITION_ID_COUNTER,
};
use crate::ContractError;

/// Fills a position. If the position already exists, it will be expanded. Otherwise, a new position is created.
pub(crate) fn fill_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    identifier: Option<String>,
    lp_asset: Asset,
    unlocking_duration: u64,
    receiver: Option<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // validate unlocking duration
    validate_unlocking_duration(&config, unlocking_duration)?;

    let mut messages: Vec<CosmosMsg> = vec![];

    //todo this will change when we remove the cw20 token support
    // ensure the lp tokens are transferred to the contract. If the LP is a cw20 token, creates
    // a transfer message
    let transfer_token_msg =
        validate_funds_sent(&deps.as_ref(), env.clone(), info.clone(), lp_asset.clone())?;

    //todo this will go away after we remove the cw20 token support
    if let Some(transfer_token_msg) = transfer_token_msg {
        messages.push(transfer_token_msg.into());
    }

    // if receiver was not specified, default to the sender of the message.
    let receiver = receiver
        .clone()
        .map(|r| deps.api.addr_validate(&r))
        .transpose()?
        .map(|receiver| MessageInfo {
            funds: info.funds.clone(),
            sender: receiver,
        })
        .unwrap_or_else(|| info.clone());

    // check if there's an existing open position with the given `identifier`
    let position = get_position(deps.storage, identifier)?;

    if let Some(mut position) = position {
        // there is a position, fill it
        position.lp_asset.amount = position.lp_asset.amount.checked_add(lp_asset.amount)?;

        POSITIONS.save(deps.storage, &position.identifier, &position)?;
    } else {
        // No position found, create a new one
        let identifier = (POSITION_ID_COUNTER
            .may_load(deps.storage)?
            .unwrap_or_default()
            + 1u64)
            .to_string();
        POSITION_ID_COUNTER.update(deps.storage, |id| Ok(id.unwrap_or_default() + 1u64))?;

        POSITIONS.save(
            deps.storage,
            &identifier,
            &Position {
                identifier,
                lp_asset,
                unlocking_duration,
                open: true,
                expiring_at: None,
                receiver: receiver.sender.clone(),
            },
        )?;
    }

    // Update weights for the LP and the user
    update_weights(deps, &receiver, &lp_asset, unlocking_duration, true)?;

    let action = match position {
        Some(_) => "expand_position",
        None => "open_position",
    };

    Ok(Response::default().add_attributes(vec![
        ("action", action.to_string()),
        ("receiver", receiver.sender.to_string()),
        ("lp_asset", lp_asset.clone().to_string()),
        ("unlocking_duration", unlocking_duration.to_string()),
    ]))
}

/// Closes an existing position
pub(crate) fn close_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    identifier: String,
    lp_asset: Option<Asset>,
) -> Result<Response, ContractError> {
    cw_utils::nonpayable(&info)?;

    //todo do this validation to see if there are pending rewards
    //query and check if the user has pending rewards
    // let rewards_query_result = get_rewards(deps.as_ref(), info.sender.clone().into_string());
    // if let Ok(rewards_response) = rewards_query_result {
    //     // can't close a position if there are pending rewards
    //     if !rewards_response.rewards.is_empty() {
    //         return Err(ContractError::PendingRewards);
    //     }
    // }

    let mut position = get_position(deps.storage, Some(identifier.clone()))?
        .ok_or(ContractError::NoPositionFound { identifier })?;

    if position.receiver != info.sender {
        return Err(ContractError::Unauthorized);
    }

    let mut attributes = vec![
        ("action", "close_position".to_string()),
        ("receiver", info.sender.to_string()),
        ("identifier", identifier.to_string()),
    ];

    // check if it's gonna be closed in full or partially
    if let Some(lp_asset) = lp_asset {
        // close position partially

        // check if the lp_asset requested to close matches the lp_asset of the position
        if position.lp_asset.info != lp_asset.info {
            return Err(ContractError::AssetMismatch);
        }

        position.lp_asset.amount = position.lp_asset.amount.saturating_sub(lp_asset.amount);

        // add the partial closing position to the storage
        let expires_at = env
            .block
            .time
            .plus_seconds(position.unlocking_duration)
            .seconds();

        let identifier = (POSITION_ID_COUNTER
            .may_load(deps.storage)?
            .unwrap_or_default()
            + 1u64)
            .to_string();
        POSITION_ID_COUNTER.update(deps.storage, |id| Ok(id.unwrap_or_default() + 1u64))?;

        let partial_position = Position {
            identifier,
            lp_asset,
            unlocking_duration: position.unlocking_duration,
            open: false,
            expiring_at: Some(expires_at),
            receiver: position.receiver.clone(),
        };
        POSITIONS.save(deps.storage, &identifier, &partial_position)?;

        attributes.push(("close_in_full", false.to_string()));
    } else {
        // close position in full
        position.open = false;
        attributes.push(("close_in_full", true.to_string()));
    }

    POSITIONS.save(deps.storage, &identifier, &position)?;

    Ok(Response::default().add_attributes(attributes))
}

/// Withdraws the given position. The position needs to have expired.
pub(crate) fn withdraw_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    identifier: String,
) -> Result<Response, ContractError> {
    cw_utils::nonpayable(&info)?;

    let position = get_position(deps.storage, Some(identifier.clone()))?
        .ok_or(ContractError::NoPositionFound { identifier })?;

    // check if this position is eligible for withdrawal
    if position.receiver != info.sender || position.open || position.expiring_at.is_none() {
        return Err(ContractError::Unauthorized);
    }

    if position.expiring_at.unwrap() > env.block.time.seconds() {
        return Err(ContractError::PositionNotExpired);
    }

    let withdraw_message = position.lp_asset.into_msg(position.receiver.clone())?;

    POSITIONS.remove(deps.storage, &identifier)?;

    Ok(Response::default()
        .add_attributes(vec![
            ("action", "withdraw_position".to_string()),
            ("receiver", info.sender.to_string()),
            ("identifier", identifier.clone().to_string()),
        ])
        .add_message(withdraw_message))
}

/// Updates the weights when managing a position. Computes what the weight is gonna be in the next epoch.
fn update_weights(
    deps: DepsMut,
    receiver: &MessageInfo,
    lp_asset: &Asset,
    unlocking_duration: u64,
    fill: bool,
) -> Result<(), ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let current_epoch = white_whale::epoch_manager::common::get_current_epoch(
        deps.as_ref(),
        config.epoch_manager_addr.clone().into_string(),
    )?;

    let weight = calculate_weight(lp_asset, unlocking_duration)?;

    let (_, mut lp_weight) = get_latest_lp_weight(deps.storage, lp_asset.info.as_bytes())?;

    if fill {
        // filling position
        lp_weight = lp_weight.checked_add(weight)?;
    } else {
        // closing position
        lp_weight = lp_weight.saturating_sub(weight)?;
    }

    LP_WEIGHTS_HISTORY.update::<_, StdError>(
        deps.storage,
        (lp_asset.info.as_bytes(), current_epoch.id + 1u64),
        |_| Ok(lp_weight),
    )?;

    // update the user's weight for this LP
    let (_, mut address_lp_weight) = get_latest_address_weight(deps.storage, &receiver.sender)?;

    if fill {
        // filling position
        address_lp_weight = address_lp_weight.checked_add(weight)?;
    } else {
        // closing position
        address_lp_weight = address_lp_weight.saturating_sub(weight)?;
    }

    ADDRESS_LP_WEIGHT_HISTORY.update::<_, StdError>(
        deps.storage,
        (&receiver.sender, current_epoch.id + 1u64),
        |_| Ok(address_lp_weight),
    )?;

    Ok(())
}
