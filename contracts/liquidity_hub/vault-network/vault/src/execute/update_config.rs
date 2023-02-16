use cosmwasm_std::{DepsMut, MessageInfo, Response};

use vault_network::vault::UpdateConfigParams;

use crate::{error::VaultError, state::CONFIG};

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    params: UpdateConfigParams,
) -> Result<Response, VaultError> {
    let UpdateConfigParams {
        flash_loan_enabled,
        withdraw_enabled,
        deposit_enabled,
        new_owner,
        new_fee_collector_addr,
        new_vault_fees,
    } = params;

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
    if let Some(new_fee_collector_addr) = new_fee_collector_addr {
        config.fee_collector_addr = deps.api.addr_validate(&new_fee_collector_addr)?;
    }
    if let Some(new_fees) = new_vault_fees {
        new_fees.is_valid()?;
        config.fees = new_fees;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![
        ("method", "update_config"),
        ("flash_loan_enabled", &config.flash_loan_enabled.to_string()),
        ("withdraw_enabled", &config.withdraw_enabled.to_string()),
        ("deposit_enabled", &config.deposit_enabled.to_string()),
        ("owner", &config.owner.into_string()),
        (
            "fee_collector_addr",
            &config.fee_collector_addr.into_string(),
        ),
    ]))
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Addr, Decimal, Response, StdError, Uint128,
    };

    use pool_network::asset::AssetInfo;
    use vault_network::vault::{Config, UpdateConfigParams};
    use white_whale::fee::{Fee, VaultFee};

    use crate::{
        contract::execute,
        error::VaultError,
        state::CONFIG,
        tests::{get_fees, mock_creator, mock_instantiate::mock_instantiate},
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
            vault_network::vault::ExecuteMsg::UpdateConfig(UpdateConfigParams {
                flash_loan_enabled: None,
                deposit_enabled: None,
                withdraw_enabled: None,
                new_owner: None,
                new_fee_collector_addr: None,
                new_vault_fees: None,
            }),
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
            fee_collector_addr: Addr::unchecked("fee_collector"),
            fees: get_fees(),
        };

        CONFIG.save(&mut deps.storage, &config).unwrap();

        let res = execute(
            deps.as_mut(),
            env,
            mock_creator(),
            vault_network::vault::ExecuteMsg::UpdateConfig(UpdateConfigParams {
                flash_loan_enabled: None,
                deposit_enabled: None,
                withdraw_enabled: None,
                new_owner: None,
                new_fee_collector_addr: None,
                new_vault_fees: None,
            }),
        )
        .unwrap();

        assert_eq!(
            res,
            Response::new().add_attributes(vec![
                ("method", "update_config"),
                ("flash_loan_enabled", "false"),
                ("withdraw_enabled", "false"),
                ("deposit_enabled", "false"),
                ("owner", &mock_creator().sender.into_string()),
                ("fee_collector_addr", "fee_collector"),
            ])
        );

        // should not have performed any changes
        let config_after = CONFIG.load(&deps.storage).unwrap();
        assert_eq!(config, config_after);
    }

    #[test]
    fn fails_if_invalid_fees() {
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
            fee_collector_addr: Addr::unchecked("fee_collector"),
            fees: get_fees(),
        };

        CONFIG.save(&mut deps.storage, &config).unwrap();

        let res = execute(
            deps.as_mut(),
            env,
            mock_creator(),
            vault_network::vault::ExecuteMsg::UpdateConfig(UpdateConfigParams {
                flash_loan_enabled: None,
                deposit_enabled: None,
                withdraw_enabled: None,
                new_owner: None,
                new_fee_collector_addr: None,
                new_vault_fees: Some(VaultFee {
                    protocol_fee: Fee {
                        share: Decimal::permille(5),
                    },
                    flash_loan_fee: Fee {
                        share: Decimal::from_ratio(Uint128::new(2), Uint128::one()),
                    },
                    burn_fee: Fee {
                        share: Decimal::zero(),
                    },
                }),
            }),
        )
        .unwrap_err();

        match res {
            VaultError::Std(e) => assert_eq!(e, StdError::generic_err("Invalid fee")),
            _ => panic!("should return Std(GenericErr -> msg: Invalid fee)"),
        }
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
            fee_collector_addr: Addr::unchecked("fee_collector"),
            fees: get_fees(),
        };

        CONFIG.save(&mut deps.storage, &config).unwrap();

        let new_fee = VaultFee {
            flash_loan_fee: Fee {
                share: Decimal::from_ratio(100u128, 1000u128),
            },
            protocol_fee: Fee {
                share: Decimal::from_ratio(100u128, 1000u128),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
        };

        let res = execute(
            deps.as_mut(),
            env,
            mock_creator(),
            vault_network::vault::ExecuteMsg::UpdateConfig(UpdateConfigParams {
                flash_loan_enabled: Some(true),
                deposit_enabled: Some(true),
                withdraw_enabled: Some(true),
                new_owner: Some("new_owner".to_string()),
                new_fee_collector_addr: Some("new_fee_collector".to_string()),
                new_vault_fees: Some(new_fee.clone()),
            }),
        )
        .unwrap();

        assert_eq!(
            res,
            Response::new().add_attributes(vec![
                ("method", "update_config"),
                ("flash_loan_enabled", "true"),
                ("withdraw_enabled", "true"),
                ("deposit_enabled", "true"),
                ("owner", "new_owner"),
                ("fee_collector_addr", "new_fee_collector"),
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
                fee_collector_addr: Addr::unchecked("new_fee_collector"),
                fees: new_fee,
            }
        );
    }
}
