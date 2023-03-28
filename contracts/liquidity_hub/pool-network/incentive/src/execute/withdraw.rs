use cosmwasm_std::{to_binary, DepsMut, Env, MessageInfo, Response, StdError, Uint128, WasmMsg};

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
        return Ok(Response::new().add_message(WasmMsg::Execute {
            contract_addr: deps.api.addr_humanize(&config.lp_address)?.into_string(),
            msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                recipient: env.contract.address.into_string(),
                amount: return_token_count,
            })?,
            funds: vec![],
        }));
    }

    // there was no positions we closed
    Ok(Response::default())
}
