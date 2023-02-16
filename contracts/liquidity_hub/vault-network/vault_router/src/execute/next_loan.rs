use cosmwasm_std::{to_binary, Addr, CosmosMsg, DepsMut, Env, MessageInfo, Response, WasmMsg};

use pool_network::asset::{Asset, AssetInfo};
use vault_network::vault_router::ExecuteMsg;

use crate::err::{StdResult, VaultRouterError};
use crate::state::CONFIG;

#[allow(clippy::too_many_arguments)]
pub fn next_loan(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mut payload: Vec<CosmosMsg>,
    initiator: Addr,
    source_vault: String,
    source_vault_asset: AssetInfo,
    to_loan: Vec<(String, Asset)>,
    loaned_assets: Vec<(String, Asset)>,
) -> StdResult<Response> {
    // check that the source vault is executing this message and it is a vault created by the WW vault factory
    let config = CONFIG.load(deps.storage)?;

    let Some(queried_vault) = deps.querier.query_wasm_smart::<Option<String>>(
        config.vault_factory,
        &vault_network::vault_factory::QueryMsg::Vault {
            asset_info: source_vault_asset,
        },
    )? else {
        return Err(VaultRouterError::Unauthorized {});
    };

    let validated_source_vault = deps.api.addr_validate(&source_vault)?;

    if info.sender != validated_source_vault
        || deps.api.addr_validate(&queried_vault)? != validated_source_vault
    {
        return Err(VaultRouterError::Unauthorized {});
    }

    let messages = match to_loan.split_first() {
        Some(((vault, asset), loans)) => {
            // loan next asset
            vec![WasmMsg::Execute {
                contract_addr: vault.clone(),
                funds: vec![],
                msg: to_binary(&vault_network::vault::ExecuteMsg::FlashLoan {
                    amount: asset.amount,
                    msg: to_binary(&ExecuteMsg::NextLoan {
                        initiator,
                        source_vault: vault.to_string(),
                        source_vault_asset_info: asset.info.clone(),
                        to_loan: loans.to_vec(),
                        payload,
                        loaned_assets,
                    })?,
                })?,
            }
            .into()]
        }
        None => {
            payload.push(
                // pay back all the loans at the end
                WasmMsg::Execute {
                    contract_addr: env.contract.address.to_string(),
                    funds: vec![],
                    msg: to_binary(&ExecuteMsg::CompleteLoan {
                        initiator,
                        loaned_assets,
                    })?,
                }
                .into(),
            );

            payload
        }
    };

    Ok(Response::new()
        .add_messages(messages)
        .add_attributes(vec![("method", "next_loan")]))
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{coins, Addr};
    use cw_multi_test::Executor;

    use pool_network::asset::AssetInfo;
    use vault_network::vault_router::ExecuteMsg;

    use crate::err::VaultRouterError;
    use crate::tests::mock_instantiate::{app_mock_instantiate, AppInstantiateResponse};
    use crate::tests::{mock_admin, mock_app_with_balance};

    // Once the nested flashloan feature is enabled again, write proper tests covering payload verification,
    // order of execution, next_loan is called as it should with nested loans and so on.

    #[test]
    fn does_require_authorization() {
        let mut app = mock_app_with_balance(vec![(mock_admin(), coins(10_000, "uluna"))]);

        let AppInstantiateResponse {
            router_addr,
            factory_addr,
            ..
        } = app_mock_instantiate(&mut app);

        // try calling NextLoan with an unauthorized vault, i.e. one that doesn't exist on the factory
        let err = app
            .execute_contract(
                Addr::unchecked("unauthorized"),
                router_addr.clone(),
                &ExecuteMsg::NextLoan {
                    initiator: Addr::unchecked("initiator_addr"),
                    source_vault: "source_vault".to_string(),
                    source_vault_asset_info: AssetInfo::Token {
                        contract_addr: "non_existing".to_string(),
                    },
                    payload: vec![],
                    to_loan: vec![],
                    loaned_assets: vec![],
                },
                &[],
            )
            .unwrap_err();

        assert_eq!(
            err.downcast::<VaultRouterError>().unwrap(),
            VaultRouterError::Unauthorized {}
        );

        let luna_vault: Option<String> = app
            .wrap()
            .query_wasm_smart(
                factory_addr,
                &vault_network::vault_factory::QueryMsg::Vault {
                    asset_info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                },
            )
            .unwrap();

        //query address of vault contract
        let err = app
            .execute_contract(
                Addr::unchecked("unauthorized"),
                router_addr,
                &ExecuteMsg::NextLoan {
                    initiator: Addr::unchecked("initiator_addr"),
                    source_vault: luna_vault.unwrap(),
                    source_vault_asset_info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    payload: vec![],
                    to_loan: vec![],
                    loaned_assets: vec![],
                },
                &[],
            )
            .unwrap_err();

        assert_eq!(
            err.downcast::<VaultRouterError>().unwrap(),
            VaultRouterError::Unauthorized {}
        );
    }
}
