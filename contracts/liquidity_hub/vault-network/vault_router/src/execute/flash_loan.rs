use cosmwasm_std::{to_binary, CosmosMsg, DepsMut, MessageInfo, Response, WasmMsg};

use pool_network::asset::Asset;
use vault_network::vault_router::ExecuteMsg;

use crate::{
    err::{StdResult, VaultRouterError},
    state::CONFIG,
};

/// Performs a flash-loan by finding the vault addresses, loaning the assets,
/// running the messages the user wants, and finally returning the assets to the
/// vault.
pub fn flash_loan(
    deps: DepsMut,
    info: MessageInfo,
    assets: Vec<Asset>,
    msgs: Vec<CosmosMsg>,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    if assets.len() > 1 {
        return Err(VaultRouterError::NestedFlashLoansDisabled {});
    }

    // get the vaults to perform loans for
    let vaults = assets
        .into_iter()
        .map(|asset| {
            // query factory for address
            let address: Option<String> = deps.querier.query_wasm_smart(
                config.vault_factory.clone(),
                &vault_network::vault_factory::QueryMsg::Vault {
                    asset_info: asset.info.clone(),
                },
            )?;

            // return InvalidAsset if address doesn't exist
            let address = address.ok_or(VaultRouterError::InvalidAsset {
                asset: asset.clone(),
            })?;

            Ok((address, asset))
        })
        .collect::<StdResult<Vec<_>>>()?;

    let mut messages: Vec<CosmosMsg> = vec![];

    // run all the loans
    if let Some(((vault, asset), next_vaults)) = vaults.split_first() {
        messages.push(
            WasmMsg::Execute {
                contract_addr: vault.to_string(),
                msg: to_binary(&vault_network::vault::ExecuteMsg::FlashLoan {
                    amount: asset.amount,
                    msg: to_binary(&ExecuteMsg::NextLoan {
                        initiator: info.sender,
                        source_vault: vault.to_string(),
                        source_vault_asset_info: asset.info.clone(),
                        to_loan: next_vaults.to_vec(),
                        payload: msgs,
                        loaned_assets: vaults,
                    })?,
                })?,
                funds: vec![],
            }
            .into(),
        );
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attributes(vec![("method", "flash_loan")]))
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        coins, from_binary, to_binary, Attribute, BankMsg, CosmosMsg, Event, Response, Uint128,
        WasmMsg,
    };
    use cw_multi_test::Executor;

    use pool_network::asset::{Asset, AssetInfo};
    use vault_network::vault_router::ExecuteMsg;

    use crate::{
        err::VaultRouterError,
        tests::{
            create_dummy_contract, mock_admin, mock_app_with_balance, mock_creator, mock_execute,
            mock_instantiate::{app_mock_instantiate, AppInstantiateResponse},
        },
    };

    #[test]
    fn does_succeed() {
        let mut app = mock_app_with_balance(vec![(mock_admin(), coins(10_066, "uluna"))]);
        let AppInstantiateResponse { router_addr, .. } = app_mock_instantiate(&mut app);

        let dummy_contract_addr = create_dummy_contract(&mut app);
        let transfer_amount = 66u128;

        // give the dummy contract a bunch of extra stuff to pay with
        app.send_tokens(
            mock_admin(),
            dummy_contract_addr.clone(),
            &coins(transfer_amount, "uluna"),
        )
        .unwrap();

        let payload = vec![WasmMsg::Execute {
            contract_addr: dummy_contract_addr.clone().into_string(),
            msg: to_binary(&crate::tests::ExecuteMsg::Send {
                to_address: router_addr.clone(),
                amount: coins(transfer_amount, "uluna"),
            })
            .unwrap(),
            funds: vec![],
        }
        .into()];

        // run a successful flash-loan
        let res = app
            .execute_contract(
                mock_creator().sender,
                router_addr.clone(),
                &ExecuteMsg::FlashLoan {
                    assets: vec![Asset {
                        amount: Uint128::new(1_000),
                        info: AssetInfo::NativeToken {
                            denom: "uluna".to_string(),
                        },
                    }],
                    msgs: payload.clone(),
                },
                &[],
            )
            .unwrap();

        // verify payload was executed
        let payload_amount = match payload.first().unwrap() {
            CosmosMsg::Wasm(WasmMsg::Execute { msg, .. }) => {
                let msg: crate::tests::ExecuteMsg = from_binary(msg).unwrap();
                match msg {
                    crate::tests::ExecuteMsg::Send { amount, .. } => amount,
                }
            }
            _ => panic!("Unexpected message"),
        };

        let payload_event = res
            .events
            .iter()
            .find(|event| {
                event.ty == "transfer"
                    && event.attributes.contains(&Attribute {
                        key: "amount".to_string(),
                        value: payload_amount.first().unwrap().to_string(),
                    })
            })
            .unwrap()
            .clone();

        let expected_payload_event = Event::new("transfer").add_attributes(vec![
            Attribute {
                key: "recipient".to_string(),
                value: router_addr.to_string(),
            },
            Attribute {
                key: "sender".to_string(),
                value: dummy_contract_addr.to_string(),
            },
            Attribute {
                key: "amount".to_string(),
                value: payload_amount.first().unwrap().to_string(),
            },
        ]);

        assert_eq!(payload_event, expected_payload_event);
    }

    #[test]
    fn does_reject_invalid_asset() {
        let mut app = mock_app_with_balance(vec![(mock_admin(), coins(10_000, "uluna"))]);
        let AppInstantiateResponse { router_addr, .. } = app_mock_instantiate(&mut app);

        let borrow_asset = Asset {
            amount: Uint128::new(1_000),
            info: AssetInfo::NativeToken {
                denom: "ukrw".to_string(),
            },
        };

        let err: VaultRouterError = app
            .execute_contract(
                mock_creator().sender,
                router_addr,
                &ExecuteMsg::FlashLoan {
                    assets: vec![borrow_asset.clone()],
                    msgs: vec![],
                },
                &[],
            )
            .unwrap_err()
            .downcast()
            .unwrap();

        assert_eq!(
            err,
            VaultRouterError::InvalidAsset {
                asset: borrow_asset
            }
        );
    }

    #[test]
    fn does_allow_empty_loan() {
        let msgs: Vec<CosmosMsg> = vec![BankMsg::Send {
            to_address: mock_creator().sender.into_string(),
            amount: coins(1, "uluna"),
        }
        .into()];

        let (res, ..) = mock_execute(
            "vault_factory",
            ExecuteMsg::FlashLoan {
                assets: vec![],
                msgs,
            },
        );

        // should not add the messages to run unless it loaned one asset
        assert_eq!(
            res.unwrap(),
            Response::new().add_attributes(vec![("method", "flash_loan")])
        )
    }

    #[test]
    fn does_not_allow_nested_flashloans() {
        let mut app = mock_app_with_balance(vec![(mock_admin(), coins(10_000, "uluna"))]);
        let AppInstantiateResponse { router_addr, .. } = app_mock_instantiate(&mut app);

        // try borrowing multiple assets, i.e. taking out nested flashloans
        let err = app
            .execute_contract(
                mock_creator().sender,
                router_addr,
                &ExecuteMsg::FlashLoan {
                    assets: vec![
                        Asset {
                            amount: Uint128::new(1_000),
                            info: AssetInfo::NativeToken {
                                denom: "uluna".to_string(),
                            },
                        },
                        Asset {
                            amount: Uint128::new(1_000),
                            info: AssetInfo::NativeToken {
                                denom: "uluna".to_string(),
                            },
                        },
                    ],
                    msgs: vec![],
                },
                &[],
            )
            .unwrap_err();

        assert_eq!(
            err.downcast::<VaultRouterError>().unwrap(),
            VaultRouterError::NestedFlashLoansDisabled {}
        );
    }

    #[test]
    fn verify_events() {
        let mut app = mock_app_with_balance(vec![(mock_admin(), coins(10_066, "uluna"))]);
        let AppInstantiateResponse {
            router_addr,
            native_vault_addr,
            ..
        } = app_mock_instantiate(&mut app);

        let dummy_contract_addr = create_dummy_contract(&mut app);
        let transfer_amount = 66u128;
        let flashloan_amount = 1_000u128;

        // give the dummy contract a bunch of extra stuff to pay with
        app.send_tokens(
            mock_admin(),
            dummy_contract_addr.clone(),
            &coins(transfer_amount, "uluna"),
        )
        .unwrap();

        let payload = vec![WasmMsg::Execute {
            contract_addr: dummy_contract_addr.clone().into_string(),
            msg: to_binary(&crate::tests::ExecuteMsg::Send {
                to_address: router_addr.clone(),
                amount: coins(transfer_amount, "uluna"),
            })
            .unwrap(),
            funds: vec![],
        }
        .into()];

        // run a successful flash-loan
        let res = app
            .execute_contract(
                mock_creator().sender,
                router_addr.clone(),
                &ExecuteMsg::FlashLoan {
                    assets: vec![Asset {
                        amount: Uint128::new(flashloan_amount),
                        info: AssetInfo::NativeToken {
                            denom: "uluna".to_string(),
                        },
                    }],
                    msgs: payload.clone(),
                },
                &[],
            )
            .unwrap();

        let payload_amount = match payload.first().unwrap() {
            CosmosMsg::Wasm(WasmMsg::Execute { msg, .. }) => {
                let msg: crate::tests::ExecuteMsg = from_binary(msg).unwrap();
                match msg {
                    crate::tests::ExecuteMsg::Send { amount, .. } => amount,
                }
            }
            _ => panic!("Unexpected message"),
        };

        // verify messages where executed in the right order
        let events = res.events;
        let expected_events = vec![
            Event::new("execute").add_attribute("_contract_addr", router_addr.to_string()),
            Event::new("wasm").add_attributes(vec![
                Attribute {
                    key: "_contract_addr".to_string(),
                    value: router_addr.to_string(),
                },
                Attribute {
                    key: "method".to_string(),
                    value: "flash_loan".to_string(),
                },
            ]),
            Event::new("execute").add_attribute("_contract_addr", native_vault_addr.to_string()),
            Event::new("wasm").add_attributes(vec![
                Attribute {
                    key: "_contract_addr".to_string(),
                    value: native_vault_addr.to_string(),
                },
                Attribute {
                    key: "method".to_string(),
                    value: "flash_loan".to_string(),
                },
                Attribute {
                    key: "amount".to_string(),
                    value: flashloan_amount.to_string(),
                },
            ]),
            Event::new("execute").add_attribute("_contract_addr", router_addr.to_string()),
            Event::new("wasm").add_attributes(vec![
                Attribute {
                    key: "_contract_addr".to_string(),
                    value: router_addr.to_string(),
                },
                Attribute {
                    key: "method".to_string(),
                    value: "next_loan".to_string(),
                },
            ]),
            Event::new("execute").add_attribute("_contract_addr", dummy_contract_addr.to_string()),
            Event::new("transfer").add_attributes(vec![
                Attribute {
                    key: "recipient".to_string(),
                    value: router_addr.to_string(),
                },
                Attribute {
                    key: "sender".to_string(),
                    value: dummy_contract_addr.to_string(),
                },
                Attribute {
                    key: "amount".to_string(),
                    value: payload_amount.first().unwrap().to_string(),
                },
            ]),
            Event::new("execute").add_attribute("_contract_addr", router_addr.to_string()),
            Event::new("wasm").add_attributes(vec![
                Attribute {
                    key: "_contract_addr".to_string(),
                    value: router_addr.to_string(),
                },
                Attribute {
                    key: "method".to_string(),
                    value: "complete_loan".to_string(),
                },
            ]),
            Event::new("transfer").add_attributes(vec![
                Attribute {
                    key: "recipient".to_string(),
                    value: native_vault_addr.to_string(),
                },
                Attribute {
                    key: "sender".to_string(),
                    value: router_addr.to_string(),
                },
                Attribute {
                    key: "amount".to_string(),
                    value: coins(transfer_amount + flashloan_amount, "uluna")
                        .first()
                        .unwrap()
                        .to_string(),
                },
            ]),
            Event::new("execute").add_attribute("_contract_addr", native_vault_addr.to_string()),
            Event::new("wasm").add_attributes(vec![
                Attribute {
                    key: "_contract_addr".to_string(),
                    value: native_vault_addr.to_string(),
                },
                Attribute {
                    key: "method".to_string(),
                    value: "after_trade".to_string(),
                },
                Attribute {
                    key: "profit".to_string(),
                    value: "0".to_string(),
                },
                Attribute {
                    key: "protocol_fee".to_string(),
                    value: (transfer_amount / 2).to_string(),
                },
                Attribute {
                    key: "flash_loan_fee".to_string(),
                    value: (transfer_amount / 2).to_string(),
                },
                Attribute {
                    key: "burn_fee".to_string(),
                    value: 0u128.to_string(),
                },
            ]),
        ];

        assert_eq!(events, expected_events);
    }
}
