use cosmwasm_std::{wasm_execute, DepsMut, Response};

use vault_network::vault::UpdateConfigParams;

use crate::err::StdResult;

pub fn update_vault_config(
    deps: DepsMut,
    vault_addr: String,
    params: UpdateConfigParams,
) -> StdResult<Response> {
    Ok(Response::new()
        .add_message(wasm_execute(
            deps.api.addr_validate(vault_addr.as_str())?.to_string(),
            &vault_network::vault::ExecuteMsg::UpdateConfig(params),
            vec![],
        )?)
        .add_attribute("method", "update_vault_config"))
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::Addr;
    use cw_multi_test::Executor;

    use crate::{
        err::VaultFactoryError,
        tests::{get_fees, mock_app, mock_creator, mock_instantiate::app_mock_instantiate},
    };

    #[test]
    fn can_update_vault_config() {
        let mut app = mock_app();

        let factory_addr = app_mock_instantiate(&mut app);

        let asset_info = pool_network::asset::AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        };

        // create a vault
        let creator = mock_creator();

        app.execute_contract(
            creator.sender.clone(),
            factory_addr.clone(),
            &vault_network::vault_factory::ExecuteMsg::CreateVault {
                asset_info: asset_info.clone(),
                fees: get_fees(),
            },
            &[],
        )
        .unwrap();

        // get vault address
        let vault_addr: Option<Addr> = app
            .wrap()
            .query_wasm_smart(
                factory_addr.clone(),
                &vault_network::vault_factory::QueryMsg::Vault { asset_info },
            )
            .unwrap();

        let vault_config: vault_network::vault::Config = app
            .wrap()
            .query_wasm_smart(
                vault_addr.clone().unwrap_or_else(|| Addr::unchecked("")),
                &vault_network::vault::QueryMsg::Config {},
            )
            .unwrap();

        // check that flashloans are enabled
        assert!(vault_config.flash_loan_enabled);

        // disable flashloans

        app.execute_contract(
            creator.sender,
            factory_addr,
            &vault_network::vault_factory::ExecuteMsg::UpdateVaultConfig {
                vault_addr: vault_addr
                    .clone()
                    .unwrap_or_else(|| Addr::unchecked(""))
                    .to_string(),
                params: vault_network::vault::UpdateConfigParams {
                    flash_loan_enabled: Some(false),
                    deposit_enabled: None,
                    withdraw_enabled: None,
                    new_owner: None,
                    new_vault_fees: None,
                    new_fee_collector_addr: None,
                },
            },
            &[],
        )
        .unwrap();

        let vault_config: vault_network::vault::Config = app
            .wrap()
            .query_wasm_smart(
                vault_addr.unwrap_or_else(|| Addr::unchecked("")),
                &vault_network::vault::QueryMsg::Config {},
            )
            .unwrap();

        // check that flashloans are disabled
        assert!(!vault_config.flash_loan_enabled);
    }

    #[test]
    fn cannot_update_vault_config_unauthorized() {
        let mut app = mock_app();

        let factory_addr = app_mock_instantiate(&mut app);

        let asset_info = pool_network::asset::AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        };

        // create a vault
        let creator = mock_creator();

        app.execute_contract(
            creator.sender,
            factory_addr.clone(),
            &vault_network::vault_factory::ExecuteMsg::CreateVault {
                asset_info: asset_info.clone(),
                fees: get_fees(),
            },
            &[],
        )
        .unwrap();

        // get vault address
        let vault_addr: Option<Addr> = app
            .wrap()
            .query_wasm_smart(
                factory_addr.clone(),
                &vault_network::vault_factory::QueryMsg::Vault { asset_info },
            )
            .unwrap();

        // unauthorized tries updating the config of the vault

        let res = app.execute_contract(
            Addr::unchecked("unauthorized"),
            factory_addr,
            &vault_network::vault_factory::ExecuteMsg::UpdateVaultConfig {
                vault_addr: vault_addr
                    .unwrap_or_else(|| Addr::unchecked(""))
                    .to_string(),
                params: vault_network::vault::UpdateConfigParams {
                    flash_loan_enabled: None,
                    deposit_enabled: None,
                    withdraw_enabled: None,
                    new_owner: Some("new_owner".to_string()),
                    new_vault_fees: None,
                    new_fee_collector_addr: None,
                },
            },
            &[],
        );

        assert_eq!(
            res.unwrap_err()
                .root_cause()
                .downcast_ref::<VaultFactoryError>()
                .unwrap(),
            &VaultFactoryError::Unauthorized {}
        );
    }
}
