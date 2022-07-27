use cosmwasm_std::{DepsMut, MessageInfo, Response};

use crate::{
    error::{StdResult, VaultError},
    state::CONFIG,
};

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    flash_loan_enabled: Option<bool>,
    withdraw_enabled: Option<bool>,
    deposit_enabled: Option<bool>,
    new_owner: Option<String>,
) -> StdResult<Response> {
    let mut config = CONFIG.load(deps.storage)?;

    if config.owner != info.sender {
        return Err(VaultError::Unauthorized {});
    }

    // if user leaves as None, do not perform change operation
    if let Some(flash_loan_enabled) = flash_loan_enabled {
        config.flash_loan_enabled = flash_loan_enabled;
    }
    if let Some(withdraw_enabled) = withdraw_enabled {
        config.withdraw_enabled = withdraw_enabled;
    }
    if let Some(deposit_enabled) = deposit_enabled {
        config.deposit_enabled = deposit_enabled;
    }
    if let Some(new_owner) = new_owner {
        config.owner = deps.api.addr_validate(&new_owner)?;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![
        ("method", "update_config"),
        ("flash_loan_enabled", &config.flash_loan_enabled.to_string()),
        ("withdraw_enabled", &config.withdraw_enabled.to_string()),
        ("deposit_enabled", &config.deposit_enabled.to_string()),
        ("owner", &config.owner.into_string()),
    ]))
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Addr, Response,
    };
    use terraswap::asset::AssetInfo;

    use crate::{
        contract::execute,
        error::VaultError,
        state::{Config, CONFIG},
        tests::{mock_creator, mock_instantiate::mock_instantiate},
    };

    #[test]
    fn fails_on_unauthorized_change() {
        let (mut deps, env) = mock_instantiate(
            2,
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
        );

        let res = execute(
            deps.as_mut(),
            env,
            mock_info("unauthorized", &[]),
            vault_network::vault::ExecuteMsg::UpdateConfig {
                flash_loan_enabled: None,
                deposit_enabled: None,
                withdraw_enabled: None,
                new_owner: None,
            },
        );

        assert_eq!(res.unwrap_err(), VaultError::Unauthorized {});
    }

    #[test]
    fn does_not_change_if_none() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let config = Config {
            owner: mock_creator().sender,
            asset_info: AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            liquidity_token: Addr::unchecked("lp_token"),
            deposit_enabled: false,
            flash_loan_enabled: false,
            withdraw_enabled: false,
        };

        CONFIG.save(&mut deps.storage, &config).unwrap();

        let res = execute(
            deps.as_mut(),
            env,
            mock_creator(),
            vault_network::vault::ExecuteMsg::UpdateConfig {
                flash_loan_enabled: None,
                deposit_enabled: None,
                withdraw_enabled: None,
                new_owner: None,
            },
        )
        .unwrap();

        assert_eq!(
            res,
            Response::new().add_attributes(vec![
                ("method", "update_config"),
                ("flash_loan_enabled", "false"),
                ("withdraw_enabled", "false"),
                ("deposit_enabled", "false"),
                ("owner", &mock_creator().sender.into_string())
            ])
        );

        // should not have performed any changes
        let config_after = CONFIG.load(&deps.storage).unwrap();
        assert_eq!(config, config_after);
    }

    #[test]
    fn does_change() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let config = Config {
            owner: mock_creator().sender,
            asset_info: AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            liquidity_token: Addr::unchecked("lp_token"),
            deposit_enabled: false,
            flash_loan_enabled: false,
            withdraw_enabled: false,
        };

        CONFIG.save(&mut deps.storage, &config).unwrap();

        let res = execute(
            deps.as_mut(),
            env,
            mock_creator(),
            vault_network::vault::ExecuteMsg::UpdateConfig {
                flash_loan_enabled: Some(true),
                deposit_enabled: Some(true),
                withdraw_enabled: Some(true),
                new_owner: Some("new_owner".to_string()),
            },
        )
        .unwrap();

        assert_eq!(
            res,
            Response::new().add_attributes(vec![
                ("method", "update_config"),
                ("flash_loan_enabled", "true"),
                ("withdraw_enabled", "true"),
                ("deposit_enabled", "true"),
                ("owner", "new_owner")
            ])
        );

        // should not have performed any changes
        let config_after = CONFIG.load(&deps.storage).unwrap();
        assert_eq!(
            config_after,
            Config {
                owner: Addr::unchecked("new_owner"),
                liquidity_token: Addr::unchecked("lp_token"),
                asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string()
                },
                deposit_enabled: true,
                flash_loan_enabled: true,
                withdraw_enabled: true,
            }
        );
    }
}
