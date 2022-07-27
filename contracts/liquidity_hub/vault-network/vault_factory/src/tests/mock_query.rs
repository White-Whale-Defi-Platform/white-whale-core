use cosmwasm_std::{
    from_binary,
    testing::{MockApi, MockQuerier, MockStorage},
    Env, OwnedDeps,
};
use serde::de::DeserializeOwned;
use vault_network::vault_factory::QueryMsg;

use crate::contract::query;

use super::mock_instantiate::mock_instantiate;

pub fn mock_query<T>(
    vault_id: u64,
    token_id: u64,
    query_msg: QueryMsg,
) -> (T, OwnedDeps<MockStorage, MockApi, MockQuerier>, Env)
where
    T: DeserializeOwned,
{
    let (deps, env) = mock_instantiate(vault_id, token_id);

    let res = from_binary(&query(deps.as_ref(), env.clone(), query_msg).unwrap()).unwrap();

    (res, deps, env)
}
