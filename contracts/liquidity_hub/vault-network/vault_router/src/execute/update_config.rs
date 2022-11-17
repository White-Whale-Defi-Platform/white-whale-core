use cosmwasm_std::{DepsMut, MessageInfo, Response};

use crate::{
    err::{StdResult, VaultRouterError},
    state::CONFIG,
};

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: Option<String>,
    new_vault_factory_addr: Option<String>,
) -> StdResult<Response> {
    let new_config = CONFIG.update::<_, VaultRouterError>(deps.storage, |mut config| {
        // check that sender is the owner
        if info.sender != config.owner {
            return Err(VaultRouterError::Unauthorized {});
        }

        if let Some(new_owner) = new_owner {
            config.owner = deps.api.addr_validate(&new_owner)?;
        };

        if let Some(new_vault_factory_addr) = new_vault_factory_addr {
            config.vault_factory = deps.api.addr_validate(&new_vault_factory_addr)?;
        }

        Ok(config)
    })?;

    Ok(Response::new().add_attributes(vec![
        ("method", "update_config"),
        ("owner", &new_config.owner.into_string()),
        ("vault_factory", &new_config.vault_factory.into_string()),
    ]))
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{testing::mock_info, Addr, Response};
    use vault_network::vault_router::{Config, ExecuteMsg};

    use crate::{
        contract::execute,
        err::VaultRouterError,
        state::CONFIG,
        tests::{mock_creator, mock_execute, mock_instantiate::mock_instantiate},
    };

    #[test]
    fn does_require_authorized() {
        let (mut deps, env) = mock_instantiate("factory");

        let bad_actor = mock_info("bad_actor", &[]);

        let res = execute(
            deps.as_mut(),
            env,
            bad_actor.clone(),
            ExecuteMsg::UpdateConfig {
                owner: Some(bad_actor.sender.into_string()),
                vault_factory_addr: Some("new_vault_address".to_string()),
            },
        );

        assert_eq!(res.unwrap_err(), VaultRouterError::Unauthorized {});
    }

    #[test]
    fn does_update_config() {
        let new_config = Config {
            owner: Addr::unchecked("new_owner"),
            vault_factory: Addr::unchecked("new_factory"),
        };

        let (res, deps, ..) = mock_execute(
            "old_factory",
            ExecuteMsg::UpdateConfig {
                owner: Some(new_config.owner.clone().into_string()),
                vault_factory_addr: Some(new_config.vault_factory.clone().into_string()),
            },
        );

        // check that expected response was sent
        assert_eq!(
            res.unwrap(),
            Response::new().add_attributes(vec![
                ("method", "update_config"),
                ("owner", &new_config.owner.clone().into_string()),
                (
                    "vault_factory",
                    &new_config.vault_factory.clone().into_string()
                )
            ])
        );

        // check that the state was updated
        assert_eq!(CONFIG.load(&deps.storage).unwrap(), new_config);
    }

    #[test]
    fn does_not_update_ignored_fields() {
        let (res, deps, ..) = mock_execute(
            "factory",
            ExecuteMsg::UpdateConfig {
                owner: None,
                vault_factory_addr: None,
            },
        );

        // check that expected response was sent
        assert_eq!(
            res.unwrap(),
            Response::new().add_attributes(vec![
                ("method", "update_config"),
                ("owner", &mock_creator().sender.into_string()),
                ("vault_factory", "factory"),
            ])
        );

        // check that the state was not updated (should be original one)
        assert_eq!(
            CONFIG.load(&deps.storage).unwrap(),
            Config {
                owner: mock_creator().sender,
                vault_factory: Addr::unchecked("factory"),
            }
        );
    }
}
