use cosmwasm_std::{CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError, Uint128};

use white_whale_std::pool_network::incentive::OpenPosition;

use crate::state::ADDRESS_WEIGHT_HISTORY;
use crate::{
    error::ContractError,
    funds_validation::validate_funds_sent,
    helpers,
    state::{ADDRESS_WEIGHT, CONFIG, GLOBAL_WEIGHT, OPEN_POSITIONS},
    weight::calculate_weight,
};

/// Opens a position for the user with the given unbonding_duration.
pub fn open_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
    unbonding_duration: u64,
    receiver: Option<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // validate unbonding duration
    let incentive_factory_config: white_whale_std::pool_network::incentive_factory::ConfigResponse =
        deps.querier.query_wasm_smart(
            config.factory_address.into_string(),
            &white_whale_std::pool_network::incentive_factory::QueryMsg::Config {},
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
    let transfer_token_msg =
        validate_funds_sent(&deps.as_ref(), env, config.lp_asset, info.clone(), amount)?;

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
        .unwrap_or_else(|| info.clone());

    // ensure an existing position with this unbonding time doesn't exist
    let existing_position = OPEN_POSITIONS
        .may_load(deps.storage, receiver.sender.clone())?
        .unwrap_or_default()
        .into_iter()
        .find(|position| position.unbonding_duration == unbonding_duration);
    if existing_position.is_some() {
        return Err(ContractError::DuplicatePosition);
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

    // add the weight to the global weight and the user's weight
    let weight = calculate_weight(unbonding_duration, amount)?;
    GLOBAL_WEIGHT.update::<_, StdError>(deps.storage, |global_weight| {
        Ok(global_weight.checked_add(weight)?)
    })?;

    let mut user_weight = ADDRESS_WEIGHT
        .may_load(deps.storage, receiver.sender.clone())?
        .unwrap_or_default();
    user_weight = user_weight.checked_add(weight)?;
    ADDRESS_WEIGHT.save(deps.storage, receiver.sender.clone(), &user_weight)?;

    let current_epoch = helpers::get_current_epoch(deps.as_ref())?;

    ADDRESS_WEIGHT_HISTORY.update::<_, StdError>(
        deps.storage,
        (&receiver.sender, current_epoch + 1u64),
        |_| Ok(user_weight),
    )?;

    Ok(Response::default()
        .add_attributes(vec![
            ("action", "open_position".to_string()),
            ("receiver", receiver.sender.to_string()),
            ("amount", amount.to_string()),
            ("unbonding_duration", unbonding_duration.to_string()),
        ])
        .add_messages(messages))
}
