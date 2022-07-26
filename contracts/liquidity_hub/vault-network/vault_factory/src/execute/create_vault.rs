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
            admin: Some(env.contract.address.clone().into_string()),
            code_id: config.vault_id,
            msg: to_binary(&InstantiateMsg {
                owner: env.contract.address.into_string(),
                asset_info: asset_info.clone(),
                token_id: config.token_id,
            })?,
            funds: vec![],
            label: format!(
                "white whale {} vault",
                asset_info.clone().get_label(&deps.as_ref())?
            ),
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

#[cfg(test)]
mod tests {
    use cosmwasm_std::{testing::mock_info, to_binary, Addr, ReplyOn, Response, SubMsg, WasmMsg};
    use cw_multi_test::Executor;
    use vault_network::vault_factory::INSTANTIATE_VAULT_REPLY_ID;

    use crate::{
        contract::execute,
        err::VaultFactoryError,
        tests::{
            mock_app, mock_creator, mock_execute,
            mock_instantiate::{app_mock_instantiate, mock_instantiate},
            store_code::{store_cw20_token_code, store_factory_code, store_vault_code},
        },
    };

    #[test]
    fn can_create_vault() {
        let asset_info = terraswap::asset::AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        };

        // create a vault
        let (res, _, env) = mock_execute(
            5,
            6,
            vault_network::vault_factory::ExecuteMsg::CreateVault {
                asset_info: asset_info.clone(),
            },
        );

        assert_eq!(
            res.unwrap(),
            Response::new()
                .add_attribute("method", "create_vault")
                .add_submessage(SubMsg {
                    id: INSTANTIATE_VAULT_REPLY_ID,
                    reply_on: ReplyOn::Success,
                    gas_limit: None,
                    msg: WasmMsg::Instantiate {
                        admin: Some(env.contract.address.to_string()),
                        code_id: 5,
                        msg: to_binary(&vault_network::vault::InstantiateMsg {
                            owner: env.contract.address.to_string(),
                            asset_info,
                            token_id: 6,
                        })
                        .unwrap(),
                        funds: vec![],
                        label: "white whale uluna vault".to_string()
                    }
                    .into()
                })
        )
    }

    #[test]
    fn cannot_create_vault_unauthorized() {
        let asset_info = terraswap::asset::AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        };

        let (mut deps, env) = mock_instantiate(5, 6);

        // create a vault unauthorized
        let bad_actor = mock_info("not_owner", &[]);

        let res = execute(
            deps.as_mut(),
            env,
            bad_actor,
            vault_network::vault_factory::ExecuteMsg::CreateVault { asset_info },
        );

        assert_eq!(res.unwrap_err(), VaultFactoryError::Unauthorized {})
    }

    #[test]
    fn cannot_create_duplicate_asset() {
        let mut app = mock_app();

        let factory_id = store_factory_code(&mut app);
        let vault_id = store_vault_code(&mut app);
        let token_id = store_cw20_token_code(&mut app);

        let factory_addr = app_mock_instantiate(&mut app, factory_id, vault_id, token_id);

        let asset_info = terraswap::asset::AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        };

        // create a vault
        let creator = mock_creator();

        app.execute_contract(
            creator.sender.clone(),
            factory_addr.clone(),
            &vault_network::vault_factory::ExecuteMsg::CreateVault {
                asset_info: asset_info.clone(),
            },
            &[],
        )
        .unwrap();

        // get vault address
        let vault_addr: Option<Addr> = app
            .wrap()
            .query_wasm_smart(
                factory_addr.clone(),
                &vault_network::vault_factory::QueryMsg::Vault {
                    asset_info: asset_info.clone(),
                },
            )
            .unwrap();

        // create a vault again
        let res = app.execute_contract(
            creator.sender,
            factory_addr,
            &vault_network::vault_factory::ExecuteMsg::CreateVault { asset_info },
            &[],
        );

        assert_eq!(
            res.unwrap_err()
                .root_cause()
                .downcast_ref::<VaultFactoryError>()
                .unwrap(),
            &VaultFactoryError::ExistingVault {
                addr: vault_addr.unwrap()
            }
        );
    }
}
