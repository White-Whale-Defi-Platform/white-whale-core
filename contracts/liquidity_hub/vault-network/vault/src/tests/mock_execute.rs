use cosmwasm_std::{
    testing::{MockApi, MockQuerier, MockStorage},
    Env, OwnedDeps, Response,
};
use pool_network::asset::AssetInfo;

use crate::contract::execute;
use crate::error::VaultError;

use super::{mock_creator, mock_instantiate::mock_instantiate};

pub fn mock_execute(
    token_id: u64,
    asset_info: AssetInfo,
    msg: vault_network::vault::ExecuteMsg,
) -> (
    Result<Response, VaultError>,
    OwnedDeps<MockStorage, MockApi, MockQuerier>,
    Env,
) {
    let (mut deps, env) = mock_instantiate(token_id, asset_info);

    (
        execute(deps.as_mut(), env.clone(), mock_creator(), msg),
        deps,
        env,
    )
}
