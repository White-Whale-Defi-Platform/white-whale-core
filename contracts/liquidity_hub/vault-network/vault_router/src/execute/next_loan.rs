use cosmwasm_std::{to_binary, Addr, CosmosMsg, DepsMut, Env, MessageInfo, Response, WasmMsg};
use terraswap::asset::Asset;
use vault_network::vault_router::ExecuteMsg;

use crate::err::{StdResult, VaultRouterError};

#[allow(clippy::too_many_arguments)]
pub fn next_loan(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mut payload: Vec<CosmosMsg>,
    initiator: Addr,
    source_vault: String,
    to_loan: Vec<(String, Asset)>,
    loaned_assets: Vec<(String, Asset)>,
) -> StdResult<Response> {
    // check that a vault is executing this message
    if info.sender != deps.api.addr_validate(&source_vault)? {
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
    use cosmwasm_std::{
        coins, testing::mock_info, to_binary, Addr, BankMsg, Response, Uint128, WasmMsg,
    };
    use terraswap::asset::{Asset, AssetInfo};
    use vault_network::vault_router::ExecuteMsg;

    use crate::{contract::execute, tests::mock_instantiate::mock_instantiate};

    #[test]
    fn does_call_next_loan() {
        let payload = vec![BankMsg::Send {
            to_address: "actor1".to_string(),
            amount: coins(1000, "uluna"),
        }
        .into()];

        let to_loan_assets = vec![
            (
                "ukrw_vault".to_string(),
                Asset {
                    amount: Uint128::new(5_000),
                    info: AssetInfo::NativeToken {
                        denom: "ukrw".to_string(),
                    },
                },
            ),
            (
                "uluna_vault".to_string(),
                Asset {
                    amount: Uint128::new(6_000),
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                },
            ),
        ];
        let loaned_assets = vec![
            (
                "token_vault".to_string(),
                Asset {
                    amount: Uint128::new(4_000),
                    info: AssetInfo::Token {
                        contract_addr: "token_loaned".to_string(),
                    },
                },
            ),
            (
                "ukrw_vault".to_string(),
                Asset {
                    amount: Uint128::new(5_000),
                    info: AssetInfo::NativeToken {
                        denom: "ukrw".to_string(),
                    },
                },
            ),
            (
                "uluna_vault".to_string(),
                Asset {
                    amount: Uint128::new(6_000),
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                },
            ),
        ];

        let (mut deps, env) = mock_instantiate("factory_addr");
        let res = execute(
            deps.as_mut(),
            env,
            mock_info("source_vault", &[]),
            ExecuteMsg::NextLoan {
                initiator: Addr::unchecked("initiator_addr"),
                source_vault: "source_vault".to_string(),
                payload: payload.clone(),
                to_loan: to_loan_assets.clone(),
                loaned_assets: loaned_assets.clone(),
            },
        );

        assert_eq!(
            res.unwrap(),
            Response::new()
                .add_message(WasmMsg::Execute {
                    contract_addr: to_loan_assets[0].0.clone(),
                    funds: vec![],
                    msg: to_binary(&vault_network::vault::ExecuteMsg::FlashLoan {
                        amount: to_loan_assets[0].1.amount,
                        msg: to_binary(&ExecuteMsg::NextLoan {
                            initiator: Addr::unchecked("initiator_addr"),
                            to_loan: to_loan_assets[1..].to_vec(),
                            payload,
                            loaned_assets,
                            source_vault: to_loan_assets[0].0.to_string()
                        })
                        .unwrap(),
                    })
                    .unwrap(),
                })
                .add_attribute("method", "next_loan")
        );
    }

    #[test]
    fn does_call_payload() {
        let payload = vec![BankMsg::Send {
            to_address: "actor1".to_string(),
            amount: coins(1000, "uluna"),
        }
        .into()];

        let loaned_assets = vec![
            (
                "token_vault".to_string(),
                Asset {
                    amount: Uint128::new(4_000),
                    info: AssetInfo::Token {
                        contract_addr: "token_loaned".to_string(),
                    },
                },
            ),
            (
                "ukrw_vault".to_string(),
                Asset {
                    amount: Uint128::new(5_000),
                    info: AssetInfo::NativeToken {
                        denom: "ukrw".to_string(),
                    },
                },
            ),
            (
                "uluna_vault".to_string(),
                Asset {
                    amount: Uint128::new(6_000),
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                },
            ),
        ];

        let (mut deps, env) = mock_instantiate("factory_addr");
        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info("source_vault", &[]),
            ExecuteMsg::NextLoan {
                initiator: Addr::unchecked("initiator_addr"),
                source_vault: "source_vault".to_string(),
                payload: payload.clone(),
                to_loan: vec![],
                loaned_assets: loaned_assets.clone(),
            },
        );

        assert_eq!(
            res.unwrap(),
            Response::new()
                .add_messages(payload.into_iter().chain(vec![WasmMsg::Execute {
                        contract_addr: env.contract.address.to_string(),
                        funds: vec![],
                        msg: to_binary(&ExecuteMsg::CompleteLoan {
                            initiator: Addr::unchecked("initiator_addr"),
                            loaned_assets,
                        })
                        .unwrap(),
                    }.into()]))
                .add_attribute("method", "next_loan")
        );
    }
}
