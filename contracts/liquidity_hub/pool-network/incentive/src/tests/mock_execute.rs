use cosmwasm_std::{
    testing::{MockApi, MockQuerier, MockStorage},
    Env, OwnedDeps, Response,
};

use crate::{contract::execute, error::ContractError};

use super::{mock_creator, mock_instantiate::mock_instantiate};

pub fn mock_execute(
    msg: white_whale::pool_network::incentive::ExecuteMsg,
) -> (
    Result<Response, ContractError>,
    OwnedDeps<MockStorage, MockApi, MockQuerier>,
    Env,
) {
    let (mut deps, env) = mock_instantiate();

    (
        execute(deps.as_mut(), env.clone(), mock_creator(), msg),
        deps,
        env,
    )
}
