#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, coins, to_binary, Uint128, WasmMsg};
    use cw_multi_test::Executor;
    use white_whale::pool_network::asset::{Asset, AssetInfo};

    use crate::tests::{
        mock_app::mock_app_with_balance,
        mock_creator,
        mock_instantiate::{app_mock_instantiate, AppInstantiateResponse},
    };

    #[test]
    fn can_deposit() {
        let mut app = mock_app_with_balance(vec![(
            mock_creator().sender,
            vec![coin(10_000, "token_a"), coin(10_000, "token_b")],
        )]);

        let AppInstantiateResponse {
            frontend_helper,
            pair_address,
            pool_assets,
        } = app_mock_instantiate(
            &mut app,
            [
                AssetInfo::NativeToken {
                    denom: "token_a".to_string(),
                },
                AssetInfo::NativeToken {
                    denom: "token_b".to_string(),
                },
            ],
        );

        // deposit
        app.execute_contract(
            mock_creator().sender,
            frontend_helper,
            &white_whale::pool_network::frontend_helper::ExecuteMsg::Deposit {
                pair_address: pair_address.into_string(),
                assets: [
                    Asset {
                        info: pool_assets[0].clone(),
                        amount: Uint128::new(5_000),
                    },
                    Asset {
                        info: pool_assets[1].clone(),
                        amount: Uint128::new(5_000),
                    },
                ],
                slippage_tolerance: None,
                unbonding_duration: 86400,
            },
            &[coin(5_000, "token_a"), coin(5_000, "token_b")],
        )
        .unwrap();
    }

    #[test]
    fn can_deposit_token() {
        let mut app =
            mock_app_with_balance(vec![(mock_creator().sender, vec![coin(10_000, "token_a")])]);

        let AppInstantiateResponse {
            frontend_helper,
            pair_address,
            pool_assets,
        } = app_mock_instantiate(
            &mut app,
            [
                AssetInfo::NativeToken {
                    denom: "token_a".to_string(),
                },
                AssetInfo::Token {
                    contract_addr: "token b".to_string(),
                },
            ],
        );

        // deposit
        app.execute_multi(
            mock_creator().sender,
            vec![
                WasmMsg::Execute {
                    contract_addr: pool_assets[1].to_string(),
                    msg: to_binary(&cw20::Cw20ExecuteMsg::IncreaseAllowance {
                        spender: frontend_helper.clone().into_string(),
                        amount: Uint128::new(5_000),
                        expires: None,
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into(),
                WasmMsg::Execute {
                    contract_addr: frontend_helper.into_string(),
                    msg: to_binary(
                        &white_whale::pool_network::frontend_helper::ExecuteMsg::Deposit {
                            pair_address: pair_address.into_string(),
                            assets: [
                                Asset {
                                    info: pool_assets[0].clone(),
                                    amount: Uint128::new(10_000),
                                },
                                Asset {
                                    info: pool_assets[1].clone(),
                                    amount: Uint128::new(5_000),
                                },
                            ],
                            slippage_tolerance: None,
                            unbonding_duration: 86400,
                        },
                    )
                    .unwrap(),
                    funds: coins(10_000, "token_a"), // don't send token_b to see how response is
                }
                .into(),
            ],
        )
        .unwrap();
    }
}
