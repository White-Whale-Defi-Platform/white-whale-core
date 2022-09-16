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
    use cosmwasm_std::{from_binary, testing::mock_info, Addr, Response};
    use vault_network::vault_factory::{ExecuteMsg, QueryMsg};

    use crate::{
        contract::{execute, query},
        err::VaultRouterError,
        state::{Config, CONFIG},
        tests::{mock_creator, mock_execute, mock_instantiate::mock_instantiate},
    };

    #[test]
    fn does_update_owner() {
        let (res, deps, env) = mock_execute(
            1,
            2,
            ExecuteMsg::UpdateConfig {
                owner: Some("other_acc".to_string()),
                fee_collector_addr: None,
                vault_id: None,
                token_id: None,
            },
        );

        // check response
        assert_eq!(
            res.unwrap(),
            Response::new().add_attributes(vec![
                ("method", "update_config"),
                ("owner", "other_acc"),
                ("fee_collector_addr", "fee_collector"),
                ("vault_id", "1"),
                ("token_id", "2")
            ])
        );

        // check query
        let config: Config =
            from_binary(&query(deps.as_ref(), env, QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(config.owner, Addr::unchecked("other_acc"));

        // check storage
        let config = CONFIG.load(&deps.storage).unwrap();
        assert_eq!(config.owner, Addr::unchecked("other_acc"));
    }

    #[test]
    fn does_update_fee_collector_addr() {
        let (res, deps, env) = mock_execute(
            1,
            2,
            ExecuteMsg::UpdateConfig {
                owner: None,
                fee_collector_addr: Some("other_acc".to_string()),
                vault_id: None,
                token_id: None,
            },
        );

        // check response
        assert_eq!(
            res.unwrap(),
            Response::new().add_attributes(vec![
                ("method", "update_config"),
                ("owner", &mock_creator().sender.into_string()),
                ("fee_collector_addr", "other_acc"),
                ("vault_id", "1"),
                ("token_id", "2")
            ])
        );

        // check query
        let config: Config =
            from_binary(&query(deps.as_ref(), env, QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(config.fee_collector_addr, Addr::unchecked("other_acc"));

        // check storage
        let config = CONFIG.load(&deps.storage).unwrap();
        assert_eq!(config.fee_collector_addr, Addr::unchecked("other_acc"));
    }

    #[test]
    fn does_update_vault_and_token_ids() {
        let (res, deps, env) = mock_execute(
            1,
            2,
            ExecuteMsg::UpdateConfig {
                owner: None,
                fee_collector_addr: None,
                vault_id: Some(3u64),
                token_id: Some(4u64),
            },
        );

        // check response
        assert_eq!(
            res.unwrap(),
            Response::new().add_attributes(vec![
                ("method", "update_config"),
                ("owner", &mock_creator().sender.to_string()),
                ("fee_collector_addr", "fee_collector"),
                ("vault_id", "3"),
                ("token_id", "4")
            ])
        );

        // check query
        let desired_config = Config {
            fee_collector_addr: Addr::unchecked("fee_collector"),
            owner: mock_creator().sender,
            vault_id: 3,
            token_id: 4,
        };

        let config: Config =
            from_binary(&query(deps.as_ref(), env, QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(config, desired_config);

        // check storage
        let config = CONFIG.load(&deps.storage).unwrap();
        assert_eq!(config, desired_config);
    }

    #[test]
    fn does_allow_empty_update() {
        let (res, deps, env) = mock_execute(
            1,
            2,
            ExecuteMsg::UpdateConfig {
                owner: None,
                fee_collector_addr: None,
                vault_id: None,
                token_id: None,
            },
        );

        // check response
        assert_eq!(
            res.unwrap(),
            Response::new().add_attributes(vec![
                ("method", "update_config"),
                ("owner", &mock_creator().sender.to_string()),
                ("fee_collector_addr", "fee_collector"),
                ("vault_id", "1"),
                ("token_id", "2")
            ])
        );

        // check query
        let desired_config = Config {
            fee_collector_addr: Addr::unchecked("fee_collector"),
            owner: mock_creator().sender,
            vault_id: 1,
            token_id: 2,
        };

        let config: Config =
            from_binary(&query(deps.as_ref(), env, QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(config, desired_config);

        // check storage
        let config = CONFIG.load(&deps.storage).unwrap();
        assert_eq!(config, desired_config);
    }

    #[test]
    fn unauthorized_update_errors() {
        let (mut deps, env) = mock_instantiate(1, 2);

        let unauthorized_sender = mock_info("bad_actor", &[]);

        let res = execute(
            deps.as_mut(),
            env,
            unauthorized_sender.clone(),
            ExecuteMsg::UpdateConfig {
                owner: Some(unauthorized_sender.sender.clone().into_string()),
                fee_collector_addr: Some(unauthorized_sender.sender.into_string()),
                vault_id: None,
                token_id: None,
            },
        )
        .unwrap_err();
        assert_eq!(res, VaultRouterError::Unauthorized {});
    }
}
