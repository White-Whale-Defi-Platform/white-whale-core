use std::collections::HashMap;

use cosmwasm_std::{CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError, Storage, Uint128};

use white_whale::incentive_manager::{
    Curve, Incentive, IncentiveParams, DEFAULT_INCENTIVE_DURATION,
};

use crate::helpers::{
    assert_incentive_asset, assert_incentive_epochs, process_incentive_creation_fee,
};
use crate::manager::MIN_INCENTIVE_AMOUNT;
use crate::state::{
    get_incentive_by_identifier, get_incentives_by_lp_asset, CONFIG, INCENTIVES, INCENTIVE_COUNTER,
};
use crate::ContractError;

/// Creates an incentive with the given params
pub(crate) fn create_incentive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mut params: IncentiveParams,
) -> Result<Response, ContractError> {
    // check if there are any expired incentives for this LP asset
    let config = CONFIG.load(deps.storage)?;
    let incentives = get_incentives_by_lp_asset(
        deps.storage,
        &params.lp_asset,
        None,
        Some(config.max_concurrent_incentives),
    )?;

    let current_epoch = white_whale::epoch_manager::common::get_current_epoch(
        deps.as_ref(),
        config.epoch_manager_addr.clone().into_string(),
    )?;

    let (expired_incentives, incentives): (Vec<_>, Vec<_>) = incentives
        .into_iter()
        .partition(|incentive| incentive.is_expired(current_epoch));

    let mut messages: Vec<CosmosMsg> = vec![];

    // close expired incentives if there are any
    if !expired_incentives.is_empty() {
        messages.append(&mut close_incentives(deps.storage, expired_incentives)?);
    }

    // check if more incentives can be created for this particular LP asset
    if incentives.len() == config.max_concurrent_incentives as usize {
        return Err(ContractError::TooManyIncentives {
            max: config.max_concurrent_incentives,
        });
    }

    // check the incentive is being created with a valid amount
    if params.incentive_asset.amount < MIN_INCENTIVE_AMOUNT {
        return Err(ContractError::InvalidIncentiveAmount {
            min: MIN_INCENTIVE_AMOUNT.u128(),
        });
    }

    let incentive_creation_fee = config.create_incentive_fee.clone();

    if incentive_creation_fee.amount != Uint128::zero() {
        // verify the fee to create an incentive is being paid
        messages.append(&mut process_incentive_creation_fee(
            &config,
            &info,
            &incentive_creation_fee,
            &mut params,
        )?);
    }

    // verify the incentive asset was sent
    messages.append(&mut assert_incentive_asset(
        deps.as_ref(),
        &env,
        &info,
        &incentive_creation_fee,
        &mut params,
    )?);

    // assert epoch params are correctly set
    let (start_epoch, end_epoch) = assert_incentive_epochs(
        &params,
        current_epoch,
        u64::from(config.max_incentive_epoch_buffer),
    )?;

    // create incentive identifier
    let incentive_id = INCENTIVE_COUNTER
        .update::<_, StdError>(deps.storage, |current_id| Ok(current_id + 1u64))?;
    let incentive_identifier = params
        .incentive_indentifier
        .unwrap_or(incentive_id.to_string());

    // make sure another incentive with the same identifier doesn't exist
    match get_incentive_by_identifier(deps.storage, &incentive_identifier) {
        Ok(_) => return Err(ContractError::IncentiveAlreadyExists {}),
        Err(_) => {} // the incentive does not exist, all good, continue
    }

    // create the incentive
    let incentive = Incentive {
        incentive_identifier,
        start_epoch,
        end_epoch,
        emitted_tokens: HashMap::new(),
        curve: params.curve.unwrap_or(Curve::Linear),
        incentive_asset: params.incentive_asset,
        lp_asset: params.lp_asset,
        incentive_creator: info.sender,
        claimed_amount: Uint128::zero(),
        asset_history: Default::default(),
    };

    Ok(Response::default().add_attributes(vec![
        ("action", "create_incentive".to_string()),
        ("incentive_creator", incentive.incentive_creator.to_string()),
        ("incentive_identifier", incentive.incentive_identifier),
        ("start_epoch", incentive.start_epoch.to_string()),
        ("end_epoch", incentive.end_epoch.to_string()),
        ("curve", incentive.curve.to_string()),
        ("incentive_asset", incentive.incentive_asset.to_string()),
        ("lp_asset", incentive.lp_asset.to_string()),
    ]))
}

/// Closes an incentive. If the incentive has expired, anyone can close it. Otherwise, only the
/// incentive creator or the owner of the contract can close an incentive.
pub(crate) fn close_incentive(
    deps: DepsMut,
    info: MessageInfo,
    incentive_identifier: String,
) -> Result<Response, ContractError> {
    // validate that user is allowed to close the incentive. Only the incentive creator or the owner of the contract can close an incentive
    let config = CONFIG.load(deps.storage)?;
    let current_epoch = white_whale::epoch_manager::common::get_current_epoch(
        deps.as_ref(),
        config.epoch_manager_addr.into_string(),
    )?;

    let mut incentive = get_incentive_by_identifier(deps.storage, &incentive_identifier)?;

    if !(!incentive.is_expired(current_epoch)
        && (incentive.incentive_creator == info.sender
            || cw_ownable::is_owner(deps.storage, &info.sender)?))
    {
        return Err(ContractError::Unauthorized {});
    }

    Ok(Response::default()
        .add_messages(close_incentives(deps.storage, vec![incentive])?)
        .add_attributes(vec![
            ("action", "close_incentive".to_string()),
            ("incentive_identifier", incentive_identifier),
        ]))
}

/// Closes a list of incentives. Does not validate the sender, do so before calling this function.
pub(crate) fn close_incentives(
    storage: &mut dyn Storage,
    incentives: Vec<Incentive>,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let mut messages: Vec<CosmosMsg> = vec![];

    for mut incentive in incentives {
        // remove the incentive from the storage
        INCENTIVES.remove(storage, incentive.incentive_identifier.clone())?;

        // return the available asset, i.e. the amount that hasn't been claimed
        incentive.incentive_asset.amount = incentive
            .incentive_asset
            .amount
            .saturating_sub(incentive.claimed_amount);

        messages.push(
            incentive
                .incentive_asset
                .into_msg(incentive.incentive_creator)?,
        );
    }

    Ok(messages)
}

/// Expands an incentive with the given params
pub(crate) fn expand_incentive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    params: IncentiveParams,
) -> Result<Response, ContractError> {
    Ok(Response::default())
}
