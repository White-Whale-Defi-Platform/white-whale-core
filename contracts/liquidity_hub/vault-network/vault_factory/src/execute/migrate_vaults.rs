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
    use cosmwasm_std::{testing::mock_info, Attribute};

    use crate::{
        contract::execute,
        err::VaultFactoryError,
        tests::{mock_creator, mock_instantiate::mock_instantiate},
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
}
