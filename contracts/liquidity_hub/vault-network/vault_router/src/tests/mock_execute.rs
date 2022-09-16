use cosmwasm_std::{
    testing::{MockApi, MockQuerier, MockStorage},
    Env, OwnedDeps, Response,
};

use crate::{contract::execute, err::StdResult};

use super::{mock_creator, mock_instantiate::mock_instantiate};

pub fn mock_execute(
    vault_id: u64,
    token_id: u64,
    msg: vault_network::vault_factory::ExecuteMsg,
) -> (
    StdResult<Response>,
    OwnedDeps<MockStorage, MockApi, MockQuerier>,
    Env,
) {
    let (mut deps, env) = mock_instantiate(vault_id, token_id);

    (
        execute(deps.as_mut(), env.clone(), mock_creator(), msg),
        deps,
        env,
    )
}
