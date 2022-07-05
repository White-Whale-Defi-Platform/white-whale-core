use cosmwasm_std::{to_binary, DepsMut, Env, MessageInfo, ReplyOn, Response, SubMsg, WasmMsg};
use terraswap::asset::AssetInfo;
use vault_network::{vault::InstantiateMsg, vault_factory::INSTANTIATE_VAULT_REPLY_ID};

use crate::{
    asset::AssetReference,
    err::{StdResult, VaultFactoryError},
    state::{CONFIG, TMP_VAULT_ASSET, VAULTS},
};

pub fn create_vault(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset_info: AssetInfo,
) -> StdResult<Response> {
    // check that owner is creating vault
    let config = CONFIG.load(deps.storage)?;
    if config.owner != info.sender {
        return Err(VaultFactoryError::Unauthorized {});
    }

    // check that existing vault does not exist
    let existing_addr = VAULTS.may_load(deps.storage, asset_info.get_reference())?;
    if let Some(addr) = existing_addr {
        return Err(VaultFactoryError::ExistingVault { addr });
    }

    // create a new vault
    let vault_instantiate_msg: SubMsg = SubMsg {
        id: INSTANTIATE_VAULT_REPLY_ID,
        msg: WasmMsg::Instantiate {
            admin: Some(env.contract.address),
            code_id: config.vault_id,
            msg: to_binary(&InstantiateMsg {
                owner: env.contract.address.into_string(),
                asset_info: asset_info.clone(),
            })?,
            funds: vec![],
            label: "white whale vault".to_string(),
        }
        .into(),
        gas_limit: None,
        reply_on: ReplyOn::Success,
    };

    // store asset for use in reply callback
    TMP_VAULT_ASSET.save(deps.storage, &asset_info.get_reference().to_vec())?;

    Ok(Response::new()
        .add_submessage(vault_instantiate_msg)
        .add_attributes(vec![("method", "create_vault")]))
}
