#[cfg(test)]
mod tests {
    use crate::error::ContractError;
    use cosmwasm_std::{coin, coins, to_json_binary, Addr, Uint128, WasmMsg};
    use cw_multi_test::Executor;
    use white_whale_std::pool_network::asset::{Asset, AssetInfo};
    use white_whale_std::pool_network::frontend_helper::ConfigResponse;
    use white_whale_std::pool_network::incentive::{PositionsResponse, QueryPosition};
    use white_whale_std::pool_network::incentive_factory::IncentiveResponse;

    use crate::tests::mock_app::mock_app;
    use crate::tests::mock_info::{mock_admin, mock_alice, mock_creator};
    use crate::tests::{
        mock_app::mock_app_with_balance,
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
            lp_token,
            pool_assets,
            incentive_factory,
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
            &white_whale_std::pool_network::frontend_helper::ExecuteMsg::Deposit {
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

        let incentive_addr: IncentiveResponse = app
            .wrap()
            .query_wasm_smart(
                incentive_factory,
                &white_whale_std::pool_network::incentive_factory::QueryMsg::Incentive {
                    lp_asset: lp_token,
                },
            )
            .unwrap();

        let positions_response: PositionsResponse = app
            .wrap()
            .query_wasm_smart(
                incentive_addr.unwrap(),
                &white_whale_std::pool_network::incentive::QueryMsg::Positions {
                    address: mock_creator().sender.to_string(),
                },
            )
            .unwrap();

        let positions: Vec<QueryPosition> = positions_response.positions;
        let position = positions.first().unwrap();
        assert_eq!(positions.len(), 1);

        match position {
            QueryPosition::OpenPosition {
                amount,
                unbonding_duration,
                ..
            } => {
                assert_eq!(amount, &Uint128::new(4_000)); // 5_000 - 1000 that is assigned to the pool as min lp mint (first deposit ever)
                assert_eq!(unbonding_duration, &86400);
            }
            QueryPosition::ClosedPosition { .. } => panic!("position should be open"),
        }
    }

    #[test]
    fn can_deposit_token() {
        let mut app =
            mock_app_with_balance(vec![(mock_creator().sender, vec![coin(10_000, "token_a")])]);

        let AppInstantiateResponse {
            frontend_helper,
            pair_address,
            lp_token,
            pool_assets,
            incentive_factory,
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
                    msg: to_json_binary(&cw20::Cw20ExecuteMsg::IncreaseAllowance {
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
                    msg: to_json_binary(
                        &white_whale_std::pool_network::frontend_helper::ExecuteMsg::Deposit {
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

        let incentive_addr: IncentiveResponse = app
            .wrap()
            .query_wasm_smart(
                incentive_factory,
                &white_whale_std::pool_network::incentive_factory::QueryMsg::Incentive {
                    lp_asset: lp_token,
                },
            )
            .unwrap();

        let positions_response: PositionsResponse = app
            .wrap()
            .query_wasm_smart(
                incentive_addr.unwrap(),
                &white_whale_std::pool_network::incentive::QueryMsg::Positions {
                    address: mock_creator().sender.to_string(),
                },
            )
            .unwrap();

        let positions: Vec<QueryPosition> = positions_response.positions;
        let position = positions.first().unwrap();
        assert_eq!(positions.len(), 1);

        match position {
            QueryPosition::OpenPosition {
                unbonding_duration, ..
            } => {
                assert_eq!(unbonding_duration, &86400);
            }
            QueryPosition::ClosedPosition { .. } => panic!("position should be open"),
        }
    }

    //write test to update the config of the contract
    #[test]
    fn update_config() {
        let mut app = mock_app();

        let AppInstantiateResponse {
            frontend_helper, ..
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

        let config: ConfigResponse = app
            .wrap()
            .query_wasm_smart(
                frontend_helper.clone(),
                &white_whale_std::pool_network::frontend_helper::QueryMsg::Config {},
            )
            .unwrap();

        assert_eq!(config.owner, mock_admin().sender);

        // try to update config
        let result = app.execute_contract(
            mock_alice().sender,
            frontend_helper.clone(),
            &white_whale_std::pool_network::frontend_helper::ExecuteMsg::UpdateConfig {
                incentive_factory_addr: Some("new_factory".to_string()),
                owner: Some("new_owner".to_string()),
            },
            &vec![],
        );

        let err = result.unwrap_err().downcast::<ContractError>().unwrap();

        match err {
            ContractError::Unauthorized { .. } => {}
            _ => panic!("Wrong error type, should return ContractError::Unauthorized"),
        }

        app.execute_contract(
            mock_admin().sender,
            frontend_helper.clone(),
            &white_whale_std::pool_network::frontend_helper::ExecuteMsg::UpdateConfig {
                incentive_factory_addr: Some("new_factory".to_string()),
                owner: Some("new_owner".to_string()),
            },
            &vec![],
        )
        .unwrap();

        let new_config: ConfigResponse = app
            .wrap()
            .query_wasm_smart(
                frontend_helper,
                &white_whale_std::pool_network::frontend_helper::QueryMsg::Config {},
            )
            .unwrap();

        assert_eq!(new_config.owner, Addr::unchecked("new_owner"));
        assert_ne!(
            new_config.incentive_factory_addr,
            config.incentive_factory_addr
        );
        assert_eq!(
            new_config.incentive_factory_addr,
            Addr::unchecked("new_factory")
        );
    }
}
