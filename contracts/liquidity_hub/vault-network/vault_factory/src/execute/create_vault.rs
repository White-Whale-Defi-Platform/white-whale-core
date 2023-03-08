use cosmwasm_std::{to_binary, DepsMut, Env, ReplyOn, Response, SubMsg, WasmMsg};
use pool_network::asset::AssetInfo;
use vault_network::{vault::InstantiateMsg, vault_factory::INSTANTIATE_VAULT_REPLY_ID};
use white_whale::fee::VaultFee;

use crate::{
    asset::AssetReference,
    err::{StdResult, VaultFactoryError},
    state::{CONFIG, TMP_VAULT_ASSET, VAULTS},
};

pub fn create_vault(
    deps: DepsMut,
    env: Env,
    asset_info: AssetInfo,
    fees: VaultFee,
) -> StdResult<Response> {
    // check that owner is creating vault
    let config = CONFIG.load(deps.storage)?;

    // check that existing vault does not exist
    let existing_addr = VAULTS.may_load(deps.storage, asset_info.get_reference())?;
    if let Some((addr, _)) = existing_addr {
        return Err(VaultFactoryError::ExistingVault { addr });
    }

    // check the fees are valid
    fees.flash_loan_fee.is_valid()?;
    fees.protocol_fee.is_valid()?;

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
                fee_collector_addr: config.fee_collector_addr.into_string(),
                vault_fees: fees,
            })?,
            funds: vec![],
            label: format!(
                "White Whale {} Vault",
                asset_info.clone().get_label(&deps.as_ref())?
            ),
        }
        .into(),
        gas_limit: None,
        reply_on: ReplyOn::Success,
    };

    // store asset for use in reply callback
    TMP_VAULT_ASSET.save(
        deps.storage,
        &(asset_info.get_reference().to_vec(), asset_info),
    )?;

    Ok(Response::new()
        .add_submessage(vault_instantiate_msg)
        .add_attributes(vec![("method", "create_vault")]))
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        testing::mock_info, to_binary, Addr, Decimal, ReplyOn, Response, StdError, SubMsg, WasmMsg,
    };
    use cw_multi_test::Executor;
    use pool_network::asset::AssetInfo;
    use vault_network::vault_factory::INSTANTIATE_VAULT_REPLY_ID;
    use white_whale::fee::{Fee, VaultFee};

    use crate::{
        contract::execute,
        err::VaultFactoryError,
        tests::{
            get_fees, mock_app, mock_creator, mock_execute,
            mock_instantiate::{app_mock_instantiate, mock_instantiate},
        },
    };

    #[test]
    fn can_create_vault() {
        let asset_info = pool_network::asset::AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        };

        // create a vault
        let (res, _, env) = mock_execute(
            5,
            6,
            vault_network::vault_factory::ExecuteMsg::CreateVault {
                asset_info: asset_info.clone(),
                fees: get_fees(),
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
                            vault_fees: get_fees(),
                            fee_collector_addr: "fee_collector".to_string()
                        })
                        .unwrap(),
                        funds: vec![],
                        label: "White Whale uluna Vault".to_string()
                    }
                    .into()
                })
        )
    }

    #[test]
    fn cannot_create_vault_unauthorized() {
        let asset_info = pool_network::asset::AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        };

        let (mut deps, env) = mock_instantiate(5, 6);

        // create a vault unauthorized
        let bad_actor = mock_info("not_owner", &[]);

        let res = execute(
            deps.as_mut(),
            env,
            bad_actor,
            vault_network::vault_factory::ExecuteMsg::CreateVault {
                asset_info,
                fees: get_fees(),
            },
        );

        assert_eq!(res.unwrap_err(), VaultFactoryError::Unauthorized {})
    }

    #[test]
    fn cannot_create_duplicate_asset() {
        let mut app = mock_app();

        let factory_addr = app_mock_instantiate(&mut app);

        let asset_info = AssetInfo::NativeToken {
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
                &vault_network::vault_factory::QueryMsg::Vault {
                    asset_info: asset_info.clone(),
                },
            )
            .unwrap();

        // create a vault again
        let res = app.execute_contract(
            creator.sender,
            factory_addr,
            &vault_network::vault_factory::ExecuteMsg::CreateVault {
                asset_info,
                fees: get_fees(),
            },
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

    #[test]
    fn does_error_if_invalid_fee() {
        let (mut deps, env) = mock_instantiate(1, 2);

        // create with bad flash loan fee
        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_creator(),
            vault_network::vault_factory::ExecuteMsg::CreateVault {
                asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                fees: VaultFee {
                    flash_loan_fee: Fee {
                        share: Decimal::percent(150),
                    },
                    protocol_fee: Fee {
                        share: Decimal::percent(30),
                    },
                    burn_fee: Fee {
                        share: Decimal::zero(),
                    },
                },
            },
        );
        assert_eq!(
            res.unwrap_err(),
            VaultFactoryError::Std(StdError::GenericErr {
                msg: "Invalid fee".to_string()
            })
        );

        // create with bad protocol fee
        let res = execute(
            deps.as_mut(),
            env,
            mock_creator(),
            vault_network::vault_factory::ExecuteMsg::CreateVault {
                asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                fees: VaultFee {
                    flash_loan_fee: Fee {
                        share: Decimal::percent(30),
                    },
                    protocol_fee: Fee {
                        share: Decimal::percent(150),
                    },
                    burn_fee: Fee {
                        share: Decimal::zero(),
                    },
                },
            },
        );
        assert_eq!(
            res.unwrap_err(),
            VaultFactoryError::Std(StdError::GenericErr {
                msg: "Invalid fee".to_string()
            })
        );
    }

    #[test]
    fn can_create_ibc_token_vault() {
        let asset_info = pool_network::asset::AssetInfo::NativeToken {
            denom: "ibc/4CD525F166D32B0132C095F353F4C6F033B0FF5C49141470D1EFDA1D63303D04"
                .to_string(),
        };

        // create a vault
        let (res, _, env) = mock_execute(
            5,
            6,
            vault_network::vault_factory::ExecuteMsg::CreateVault {
                asset_info: asset_info.clone(),
                fees: get_fees(),
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
                            vault_fees: get_fees(),
                            fee_collector_addr: "fee_collector".to_string()
                        })
                        .unwrap(),
                        funds: vec![],
                        label: "White Whale ibc/4CD5...3D04 Vault".to_string()
                    }
                    .into()
                })
        )
    }

    #[cfg(feature = "injective")]
    #[test]
    fn can_create_peggy_token_vault() {
        let asset_info = pool_network::asset::AssetInfo::NativeToken {
            denom: "peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5".to_string(),
        };

        // create a vault
        let (res, _, env) = mock_execute(
            5,
            6,
            vault_network::vault_factory::ExecuteMsg::CreateVault {
                asset_info: asset_info.clone(),
                fees: get_fees(),
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
                            vault_fees: get_fees(),
                            fee_collector_addr: "fee_collector".to_string()
                        })
                        .unwrap(),
                        funds: vec![],
                        label: "White Whale peggy0x87a...1B5 Vault".to_string()
                    }
                    .into()
                })
        )
    }
}
