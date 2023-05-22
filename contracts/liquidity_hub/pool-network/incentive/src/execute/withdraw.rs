use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdError, Uint128};
use white_whale::pool_network::asset::Asset;

use crate::{
    error::ContractError,
    state::{ADDRESS_WEIGHT, CLOSED_POSITIONS, CONFIG, GLOBAL_WEIGHT},
};

pub fn withdraw(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    // counter of how many LP tokens we must return to use and the weight to remove
    let mut return_token_count = Uint128::zero();
    let mut weight_to_remove = Uint128::zero();

    CLOSED_POSITIONS.update::<_, ContractError>(
        deps.storage,
        info.sender.clone(),
        |closed_positions| {
            let mut closed_positions = closed_positions.unwrap_or_default();

            for i in (0..closed_positions.len()).rev() {
                let position = &closed_positions[i];

                if env.block.time.seconds() > position.unbonding_timestamp {
                    // remove weight
                    // this should be the position amount, as that is the amount we didn't subtract
                    // when we closed the position
                    weight_to_remove = weight_to_remove.checked_add(position.amount)?;

                    // add return tokens to sum
                    return_token_count = return_token_count.checked_add(position.amount)?;

                    // remove position
                    closed_positions.remove(i);
                }
            }

            Ok(closed_positions)
        },
    )?;

    if !weight_to_remove.is_zero() {
        GLOBAL_WEIGHT.update::<_, StdError>(deps.storage, |global_weight| {
            Ok(global_weight.checked_sub(weight_to_remove)?)
        })?;
        ADDRESS_WEIGHT.update::<_, StdError>(deps.storage, info.sender.clone(), |user_weight| {
            Ok(user_weight
                .unwrap_or_default()
                .checked_sub(weight_to_remove)?)
        })?;
    }

    if !return_token_count.is_zero() {
        let config = CONFIG.load(deps.storage)?;

        let return_asset = Asset {
            info: config.lp_asset,
            amount: return_token_count.clone(),
        };

        return Ok(Response::default()
            .add_attributes(vec![
                ("action", "withdraw".to_string()),
                ("return_asset", return_asset.to_string()),
            ])
            .add_message(return_asset.into_msg(info.sender)?));
    }

    // there was no positions we closed
    Ok(Response::default().add_attributes(vec![
        ("action", "withdraw"),
        ("result", "no positions were closed"),
    ]))
}
