#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, Uint128};
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
        } = app_mock_instantiate(&mut app);

        // deposit
        app.execute_contract(
            mock_creator().sender,
            frontend_helper,
            &white_whale::pool_network::frontend_helper::ExecuteMsg::Deposit {
                pair_address: pair_address.into_string(),
                assets: [
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: "token_a".to_string(),
                        },
                        amount: Uint128::new(5_000),
                    },
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: "token_b".to_string(),
                        },
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
}
