use cosmwasm_std::{
    from_json,
    testing::{MockApi, MockQuerier, MockStorage},
    Env, OwnedDeps,
};
use serde::de::DeserializeOwned;
use white_whale_std::vault_network::vault_router::QueryMsg;

use crate::contract::query;

use super::mock_instantiate::mock_instantiate;

pub fn mock_query<T>(
    factory_addr: String,
    query_msg: QueryMsg,
) -> (T, OwnedDeps<MockStorage, MockApi, MockQuerier>, Env)
where
    T: DeserializeOwned,
{
    let (deps, env) = mock_instantiate(factory_addr);

    let res = from_json(query(deps.as_ref(), env.clone(), query_msg).unwrap()).unwrap();

    (res, deps, env)
}
