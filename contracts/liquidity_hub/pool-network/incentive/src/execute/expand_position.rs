use cosmwasm_std::{CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError, Uint128};

use crate::state::ADDRESS_WEIGHT_HISTORY;
use crate::{
    error::ContractError,
    funds_validation::validate_funds_sent,
    helpers,
    state::{ADDRESS_WEIGHT, CONFIG, GLOBAL_WEIGHT, OPEN_POSITIONS},
    weight::calculate_weight,
};

/// Expands a position a user already has by adding more funds to it.
pub fn expand_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
    unbonding_duration: u64,
    receiver: Option<String>,
) -> Result<Response, ContractError> {
    // first ensure that the relevant tokens were transferred to us
    let config = CONFIG.load(deps.storage)?;
    let mut messages: Vec<CosmosMsg> = vec![];

    {
        let claim_token_msg =
            validate_funds_sent(&deps.as_ref(), env, config.lp_asset, info.clone(), amount)?;

        if let Some(claim_token_msg) = claim_token_msg {
            messages.push(claim_token_msg.into());
        }
    }

    // if a receiver was specified, use them with the funds sent by the message sender
    // if not, default to the message sender
    // this has the intentional side-effect of moving `info` so it cannot be used later.
    let receiver = receiver
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?
        .map(|receiver| MessageInfo {
            funds: info.funds.clone(),
            sender: receiver,
        })
        .unwrap_or(info);

    // increase position
    OPEN_POSITIONS.update::<_, ContractError>(
        deps.storage,
        receiver.sender.clone(),
        |positions| {
            let mut positions =
                positions.ok_or(ContractError::NonExistentPosition { unbonding_duration })?;

            let pos = positions
                .iter_mut()
                .find(|position| position.unbonding_duration == unbonding_duration)
                .ok_or(ContractError::NonExistentPosition { unbonding_duration })?;

            pos.amount += amount;

            Ok(positions)
        },
    )?;

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
            ("action", "expand_position".to_string()),
            ("receiver", receiver.sender.to_string()),
            ("amount", amount.to_string()),
            ("unbonding_duration", unbonding_duration.to_string()),
        ])
        .add_messages(messages))
}
