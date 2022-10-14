use cosmwasm_std::{to_binary, Addr, CosmosMsg, DepsMut, Response, WasmMsg};

use vault_network::vault::MigrateMsg;

use crate::err::StdResult;
use crate::state::read_vaults;

pub fn migrate_vaults(
    deps: DepsMut,
    vault_addr: Option<String>,
    vault_code_id: u64,
) -> StdResult<Response> {
    // migrate only the provided vault address, otherwise migrate all vaults
    let mut res = Response::new().add_attributes(vec![
        ("method", "migrate_vaults".to_string()),
        ("code_id", vault_code_id.to_string()),
    ]);
    if let Some(vault_addr) = vault_addr {
        Ok(res
            .add_attribute("vault", vault_addr.clone())
            .add_message(migrate_vault_msg(
                deps.api.addr_validate(vault_addr.as_str())?,
                vault_code_id,
            )?))
    } else {
        let vaults = read_vaults(deps.storage, deps.api, None, Some(30u32))?;
        for vault in vaults {
            res = res
                .add_attribute("vault", &vault.clone().vault)
                .add_message(migrate_vault_msg(
                    deps.api.addr_validate(vault.vault.as_str())?,
                    vault_code_id,
                )?)
        }

        Ok(res)
    }
}

/// Creates a migrate vault message given a vault address and code id
fn migrate_vault_msg(vault_addr: Addr, code_id: u64) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Wasm(WasmMsg::Migrate {
        contract_addr: vault_addr.to_string(),
        new_code_id: code_id,
        msg: to_binary(&MigrateMsg {})?,
    }))
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{testing::mock_info, Addr, Attribute};
    use cw_multi_test::Executor;

    use crate::tests::store_code::store_vault_code;
    use crate::{
        contract::execute,
        err::VaultFactoryError,
        tests::{
            get_fees, mock_app, mock_creator,
            mock_instantiate::{app_mock_instantiate, mock_instantiate},
        },
    };

    #[test]
    fn cannot_migrate_vault_unauthorized() {
        let (mut deps, env) = mock_instantiate(5, 6);

        // migrate a vault unauthorized
        let bad_actor = mock_info("not_owner", &[]);

        let res = execute(
            deps.as_mut(),
            env,
            bad_actor,
            vault_network::vault_factory::ExecuteMsg::MigrateVaults {
                vault_addr: None,
                vault_code_id: 7,
            },
        );

        assert_eq!(res.unwrap_err(), VaultFactoryError::Unauthorized {})
    }

    #[test]
    fn can_migrate_single_vault() {
        let (mut deps, env) = mock_instantiate(5, 6);
        let creator = mock_creator();
        // migrate a vault unauthorized
        let info = mock_info(creator.sender.as_str(), &[]);

        let res = execute(
            deps.as_mut(),
            env,
            info,
            vault_network::vault_factory::ExecuteMsg::MigrateVaults {
                vault_addr: Some("outdated_vault".to_string()),
                vault_code_id: 7,
            },
        )
        .unwrap();

        let expected_attributes = vec![
            Attribute {
                key: "method".to_string(),
                value: "migrate_vaults".to_string(),
            },
            Attribute {
                key: "code_id".to_string(),
                value: "7".to_string(),
            },
            Attribute {
                key: "vault".to_string(),
                value: "outdated_vault".to_string(),
            },
        ];

        assert_eq!(res.attributes, expected_attributes);
    }

    #[test]
    fn can_migrate_multiple_vaults() {
        let mut app = mock_app();
        let creator = mock_creator();

        let factory_addr = app_mock_instantiate(&mut app);
        let new_vault_code_id = store_vault_code(&mut app);

        // create two vaults
        let asset_info_1 = terraswap::asset::AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        };

        app.execute_contract(
            creator.sender.clone(),
            factory_addr.clone(),
            &vault_network::vault_factory::ExecuteMsg::CreateVault {
                asset_info: asset_info_1.clone(),
                fees: get_fees(),
            },
            &[],
        )
        .unwrap();

        // get vault address
        let vault_addr_1: Option<Addr> = app
            .wrap()
            .query_wasm_smart(
                factory_addr.clone(),
                &vault_network::vault_factory::QueryMsg::Vault {
                    asset_info: asset_info_1.clone(),
                },
            )
            .unwrap();

        let asset_info_2 = terraswap::asset::AssetInfo::NativeToken {
            denom: "ujuno".to_string(),
        };

        app.execute_contract(
            creator.sender.clone(),
            factory_addr.clone(),
            &vault_network::vault_factory::ExecuteMsg::CreateVault {
                asset_info: asset_info_2.clone(),
                fees: get_fees(),
            },
            &[],
        )
        .unwrap();

        // get vault address
        let vault_addr_2: Option<Addr> = app
            .wrap()
            .query_wasm_smart(
                factory_addr.clone(),
                &vault_network::vault_factory::QueryMsg::Vault {
                    asset_info: asset_info_2.clone(),
                },
            )
            .unwrap();

        // migrate vaults
        let res = app
            .execute_contract(
                creator.sender.clone(),
                factory_addr.clone(),
                &vault_network::vault_factory::ExecuteMsg::MigrateVaults {
                    vault_addr: None,
                    vault_code_id: new_vault_code_id.clone(),
                },
                &[],
            )
            .unwrap();

        assert_eq!(res.events.len(), 4);

        for event in res.events {
            if event.ty == "wasm" {
                let expected_attributes = vec![
                    Attribute {
                        key: "_contract_addr".to_string(),
                        value: factory_addr.clone().to_string(),
                    },
                    Attribute {
                        key: "method".to_string(),
                        value: "migrate_vaults".to_string(),
                    },
                    Attribute {
                        key: "code_id".to_string(),
                        value: new_vault_code_id.to_string(),
                    },
                    Attribute {
                        key: "vault".to_string(),
                        value: vault_addr_2
                            .clone()
                            .unwrap_or(Addr::unchecked(""))
                            .to_string(),
                    },
                    Attribute {
                        key: "vault".to_string(),
                        value: vault_addr_1
                            .clone()
                            .unwrap_or(Addr::unchecked(""))
                            .to_string(),
                    },
                ];

                assert_eq!(event.attributes, expected_attributes);
            }
        }
    }
}
