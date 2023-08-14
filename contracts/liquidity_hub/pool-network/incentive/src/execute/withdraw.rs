use classic_bindings::TerraQuery;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Uint128};
use white_whale::pool_network::asset::Asset;

use crate::{
    error::ContractError,
    state::{CLOSED_POSITIONS, CONFIG},
};

/// Withdraws LP tokens from the contract.
pub fn withdraw(
    deps: DepsMut<TerraQuery>,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // counter of how many LP tokens we must return to use and the weight to remove
    let mut return_token_count = Uint128::zero();

    CLOSED_POSITIONS.update::<_, ContractError>(
        deps.storage,
        info.sender.clone(),
        |closed_positions| {
            let mut closed_positions = closed_positions.unwrap_or_default();

            for i in (0..closed_positions.len()).rev() {
                let position = &closed_positions[i];

                // if unbonding timestamp is in the past, it's possible to withdraw
                if env.block.time.seconds() > position.unbonding_timestamp {
                    // add return tokens to sum
                    return_token_count = return_token_count.checked_add(position.amount)?;

                    // remove position
                    closed_positions.remove(i);
                }
            }

            Ok(closed_positions)
        },
    )?;

    // if we have some tokens to return, return them
    if !return_token_count.is_zero() {
        let config = CONFIG.load(deps.storage)?;

        let return_asset = Asset {
            info: config.lp_asset,
            amount: return_token_count,
        };

        return Ok(Response::default()
            .add_attributes(vec![
                ("action", "withdraw".to_string()),
                ("return_asset", return_asset.to_string()),
            ])
            .add_message(return_asset.into_msg(&deps.querier, info.sender)?));
    }

    // there was no positions we closed
    Ok(Response::default().add_attributes(vec![
        ("action", "withdraw"),
        ("result", "no positions were closed"),
    ]))
}
