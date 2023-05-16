use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdError};
use white_whale::pool_network::incentive::{ClosedPosition, OpenPosition};

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
    // claim current position
    let claim_messages = crate::claim::claim(&mut deps, &env, &info)?;

    // remove position
    let mut open_positions = OPEN_POSITIONS
        .may_load(deps.storage, info.sender.clone())?
        .ok_or(ContractError::NonExistentPosition { unbonding_duration })?;
    let to_close_index = open_positions
        .iter()
        .position(|pos| pos.unbonding_duration == unbonding_duration)
        .ok_or(ContractError::NonExistentPosition { unbonding_duration })?;
    let to_close_position = &open_positions[to_close_index];

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
    let weight = calculate_weight(unbonding_duration, to_close_position.amount)?;
    let reduced_weight = weight.checked_sub(to_close_position.amount)?;

    GLOBAL_WEIGHT.update::<_, StdError>(deps.storage, |global_weight| {
        Ok(global_weight.checked_sub(reduced_weight)?)
    })?;
    ADDRESS_WEIGHT.update::<_, StdError>(deps.storage, info.sender.clone(), |user_weight| {
        Ok(user_weight
            .unwrap_or_default()
            .checked_sub(reduced_weight)?)
    })?;

    let closing_position: OpenPosition = open_positions[to_close_index].clone();

    open_positions.remove(to_close_index);
    OPEN_POSITIONS.save(deps.storage, info.sender, &open_positions)?;

    Ok(Response::default()
        .add_attributes(vec![
            ("action", "close_position".to_string()),
            ("closing_position", closing_position.to_string()),
        ])
        .add_messages(claim_messages))
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::Uint128;
    use cw_multi_test::Executor;
    use white_whale::pool_network::incentive::{PositionsResponse, QueryMsg};

    use crate::tests::{
        mock_app::mock_app,
        mock_creator,
        mock_instantiate::{app_mock_instantiate, AppInstantiateResponse},
    };

    #[test]
    fn can_close_position() {
        let mut app = mock_app();

        let lp_balance = Uint128::new(1_000_000u128);

        let AppInstantiateResponse {
            incentive_addr,
            lp_addr,
        } = app_mock_instantiate(&mut app, lp_balance);

        // create a new position
        app.execute_contract(
            mock_creator().sender,
            lp_addr,
            &cw20_base::msg::ExecuteMsg::IncreaseAllowance {
                spender: incentive_addr.to_string(),
                amount: lp_balance,
                expires: None,
            },
            &[],
        )
        .unwrap();

        let positions_response: PositionsResponse = app
            .wrap()
            .query_wasm_smart(
                incentive_addr.clone(),
                &QueryMsg::Positions {
                    address: "creator".to_string(),
                },
            )
            .unwrap();

        assert!(positions_response.positions.is_empty());

        app.execute_contract(
            mock_creator().sender,
            incentive_addr.clone(),
            &white_whale::pool_network::incentive::ExecuteMsg::OpenPosition {
                amount: lp_balance,
                unbonding_duration: 86400,
                receiver: None,
            },
            &[],
        )
        .unwrap();

        let positions_response: PositionsResponse = app
            .wrap()
            .query_wasm_smart(
                incentive_addr.clone(),
                &QueryMsg::Positions {
                    address: "creator".to_string(),
                },
            )
            .unwrap();

        assert_eq!(positions_response.positions.len(), 1);

        // now try to close the position
        app.execute_contract(
            mock_creator().sender,
            incentive_addr,
            &white_whale::pool_network::incentive::ExecuteMsg::ClosePosition {
                unbonding_duration: 86400,
            },
            &[],
        )
        .unwrap();
    }
}
