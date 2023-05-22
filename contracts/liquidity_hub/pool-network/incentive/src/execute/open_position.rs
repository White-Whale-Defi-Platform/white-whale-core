use cosmwasm_std::{CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError, Uint128};

use white_whale::pool_network::incentive::OpenPosition;

use crate::{
    error::ContractError,
    funds_validation::validate_funds_sent,
    state::{ADDRESS_WEIGHT, CONFIG, GLOBAL_WEIGHT, OPEN_POSITIONS},
    weight::calculate_weight,
};

pub fn open_position(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
    unbonding_duration: u64,
    receiver: Option<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // validate unbonding duration
    let incentive_factory_config: white_whale::pool_network::incentive_factory::ConfigResponse =
        deps.querier.query_wasm_smart(
            config.factory_address.into_string(),
            &white_whale::pool_network::incentive_factory::QueryMsg::Config {},
        )?;

    if unbonding_duration < incentive_factory_config.min_unbonding_duration
        || unbonding_duration > incentive_factory_config.max_unbonding_duration
    {
        return Err(ContractError::InvalidUnbondingDuration {
            min: incentive_factory_config.min_unbonding_duration,
            max: incentive_factory_config.max_unbonding_duration,
            specified: unbonding_duration,
        });
    }

    let mut messages: Vec<CosmosMsg> = vec![];

    // ensure that user gave us an allowance for the token amount
    // we check this on the message sender rather than the receiver
    let transfer_token_msg = validate_funds_sent(
        &deps.as_ref(),
        env.clone(),
        config.lp_asset,
        info.clone(),
        amount,
    )?;

    if let Some(transfer_token_msg) = transfer_token_msg {
        messages.push(transfer_token_msg.into());
    }

    // if receiver was not specified, default to the sender of the message.
    let receiver = receiver
        .map(|r| deps.api.addr_validate(&r))
        .transpose()?
        .map(|receiver| MessageInfo {
            funds: info.funds.clone(),
            sender: receiver,
        })
        .unwrap_or(info);

    // ensure an existing position with this unbonding time doesn't exist
    let existing_position = OPEN_POSITIONS
        .may_load(deps.storage, receiver.sender.clone())?
        .unwrap_or_default()
        .into_iter()
        .find(|position| position.unbonding_duration == unbonding_duration);
    if existing_position.is_some() {
        return Err(ContractError::DuplicatePosition);
    }

    // claim if the user has open positions to ensure that the user gets reward for current weight
    // rather than future weight after opening the position
    if !OPEN_POSITIONS
        .may_load(deps.storage, receiver.sender.clone())?
        .unwrap_or_default()
        .is_empty()
    {
        messages.append(&mut crate::claim::claim(&mut deps, &env, &receiver)?);
    }

    // create the new position
    OPEN_POSITIONS.update::<_, StdError>(deps.storage, receiver.sender.clone(), |positions| {
        let mut positions = positions.unwrap_or_default();

        positions.push(OpenPosition {
            amount,
            unbonding_duration,
        });

        Ok(positions)
    })?;

    // add the weight
    let weight = calculate_weight(unbonding_duration, amount)?;
    GLOBAL_WEIGHT.update::<_, StdError>(deps.storage, |global_weight| {
        Ok(global_weight.checked_add(weight)?)
    })?;
    ADDRESS_WEIGHT.update::<_, StdError>(deps.storage, receiver.sender.clone(), |user_weight| {
        Ok(user_weight.unwrap_or_default().checked_add(weight)?)
    })?;

    Ok(Response::default()
        .add_attributes(vec![
            ("action", "open_position".to_string()),
            ("receiver", receiver.sender.to_string()),
            ("amount", amount.to_string()),
            ("unbonding_duration", unbonding_duration.to_string()),
        ])
        .add_messages(messages))
}
