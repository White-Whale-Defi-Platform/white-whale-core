use classic_bindings::TerraQuery;
use cosmwasm_std::{Deps, Env};
use white_whale::pool_network::incentive::{PositionsResponse, QueryPosition};

use crate::{
    error::ContractError,
    state::{CLOSED_POSITIONS, OPEN_POSITIONS},
    weight::calculate_weight,
};

/// Gets the positions for the given address. Returns a [PositionsResponse] struct.
pub fn get_positions(
    deps: Deps<TerraQuery>,
    env: Env,
    address: String,
) -> Result<PositionsResponse, ContractError> {
    let address = deps.api.addr_validate(&address)?;

    let open_positions = OPEN_POSITIONS
        .may_load(deps.storage, address.clone())?
        .unwrap_or_default()
        .into_iter()
        .map(|position| {
            Ok(QueryPosition::OpenPosition {
                amount: position.amount,
                unbonding_duration: position.unbonding_duration,
                weight: calculate_weight(position.unbonding_duration, position.amount)?,
            })
        })
        .collect::<Result<Vec<_>, ContractError>>()?;

    let closed_positions = CLOSED_POSITIONS
        .may_load(deps.storage, address)?
        .unwrap_or_default()
        .into_iter()
        .map(|position| QueryPosition::ClosedPosition {
            amount: position.amount,
            unbonding_timestamp: position.unbonding_timestamp,
            // the weight of a closed position is equivalent to the amount of LP tokens in the position
            weight: position.amount,
        });

    Ok(PositionsResponse {
        timestamp: env.block.time.seconds(),
        positions: open_positions.into_iter().chain(closed_positions).collect(),
    })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env},
        Addr, Uint128,
    };
    use white_whale::pool_network::incentive::{ClosedPosition, OpenPosition, QueryPosition};

    use crate::{
        state::{CLOSED_POSITIONS, OPEN_POSITIONS},
        weight::calculate_weight,
    };

    use super::get_positions;

    #[test]
    fn does_handle_no_positions() {
        let deps = mock_dependencies();

        // should not error when doing empty positions
        let res = get_positions(
            deps.as_ref(),
            mock_env(),
            Addr::unchecked("creator").to_string(),
        )
        .unwrap();
        assert!(res.positions.is_empty());
    }

    #[test]
    fn does_return_timestamp() {
        let deps = mock_dependencies();

        // should return the mock_env timestamp
        let timestamp = get_positions(
            deps.as_ref(),
            mock_env(),
            Addr::unchecked("creator").to_string(),
        )
        .unwrap()
        .timestamp;
        assert_eq!(timestamp, mock_env().block.time.seconds());
    }

    #[test]
    fn does_return_open_positions() {
        let mut deps = mock_dependencies();

        let creator = Addr::unchecked("creator");

        // inject a position
        let amount = Uint128::new(1_000);

        OPEN_POSITIONS
            .save(
                deps.as_mut().storage,
                creator.clone(),
                &vec![OpenPosition {
                    amount,
                    unbonding_duration: 86_400,
                }],
            )
            .unwrap();

        let positions = get_positions(deps.as_ref(), mock_env(), creator.into_string()).unwrap();
        assert_eq!(
            positions.positions,
            vec![QueryPosition::OpenPosition {
                amount,
                unbonding_duration: 86_400,
                weight: calculate_weight(86_400, amount).unwrap()
            }]
        );
    }

    #[test]
    fn does_return_closed_positions() {
        let mut deps = mock_dependencies();

        let creator = Addr::unchecked("creator");

        // inject a position
        let amount = Uint128::new(1_000);

        CLOSED_POSITIONS
            .save(
                deps.as_mut().storage,
                creator.clone(),
                &vec![ClosedPosition {
                    amount,
                    unbonding_timestamp: 100,
                }],
            )
            .unwrap();

        let positions = get_positions(deps.as_ref(), mock_env(), creator.into_string()).unwrap();
        assert_eq!(
            positions.positions,
            vec![QueryPosition::ClosedPosition {
                amount,
                unbonding_timestamp: 100,
                weight: amount
            }]
        );
    }
}
