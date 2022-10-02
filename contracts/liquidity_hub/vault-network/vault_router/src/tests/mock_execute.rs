use cosmwasm_std::{
    testing::{MockApi, MockQuerier, MockStorage},
    Env, OwnedDeps, Response,
};

use crate::{contract::execute, err::StdResult};

use super::{mock_creator, mock_instantiate::mock_instantiate};

pub fn mock_execute<F: Into<String>>(
    factory_addr: F,
    msg: vault_network::vault_router::ExecuteMsg,
) -> (
    StdResult<Response>,
    OwnedDeps<MockStorage, MockApi, MockQuerier>,
    Env,
) {
    let (mut deps, env) = mock_instantiate(factory_addr);

    (
        execute(deps.as_mut(), env.clone(), mock_creator(), msg),
        deps,
        env,
    )
}
