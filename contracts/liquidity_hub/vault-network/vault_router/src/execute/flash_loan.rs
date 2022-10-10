use cosmwasm_std::{to_binary, CosmosMsg, DepsMut, MessageInfo, Response, WasmMsg};
use terraswap::asset::Asset;
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
    use cosmwasm_std::{coins, to_binary, BankMsg, CosmosMsg, Response, Uint128, WasmMsg};
    use cw_multi_test::Executor;
    use terraswap::asset::{Asset, AssetInfo};
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

        // give the dummy contract a bunch of extra stuff to pay with
        app.send_tokens(
            mock_admin(),
            dummy_contract_addr.clone(),
            &coins(66, "uluna"),
        )
        .unwrap();

        // run a successful flash-loan
        app.execute_contract(
            mock_creator().sender,
            router_addr.clone(),
            &ExecuteMsg::FlashLoan {
                assets: vec![Asset {
                    amount: Uint128::new(1_000),
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                }],
                msgs: vec![WasmMsg::Execute {
                    contract_addr: dummy_contract_addr.into_string(),
                    msg: to_binary(&crate::tests::ExecuteMsg::Send {
                        to_address: router_addr,
                        amount: coins(66, "uluna"),
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into()],
            },
            &[],
        )
        .unwrap();
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
}
