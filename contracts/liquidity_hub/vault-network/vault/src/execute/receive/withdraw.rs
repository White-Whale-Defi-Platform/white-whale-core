use cosmwasm_std::{
    coins, to_binary, BankMsg, CosmosMsg, Decimal, DepsMut, Env, Response, Uint128, WasmMsg,
};
use cw20::{BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg, TokenInfoResponse};

use pool_network::asset::AssetInfo;

use crate::state::COLLECTED_PROTOCOL_FEES;
use crate::{error::VaultError, state::CONFIG};

pub fn withdraw(
    deps: DepsMut,
    env: Env,
    sender: String,
    amount: Uint128,
) -> Result<Response, VaultError> {
    let config = CONFIG.load(deps.storage)?;

    // check that withdrawals are enabled
    if !config.withdraw_enabled {
        return Err(VaultError::WithdrawsDisabled {});
    }

    // parse sender
    let sender = deps.api.addr_validate(&sender)?;

    // calculate the size of vault and the amount of assets to withdraw
    let collected_protocol_fees = COLLECTED_PROTOCOL_FEES.load(deps.storage)?;
    let total_asset_amount = match &config.asset_info {
        AssetInfo::NativeToken { denom } => {
            deps.querier
                .query_balance(env.contract.address, denom)?
                .amount
        }
        AssetInfo::Token { contract_addr } => {
            let balance: BalanceResponse = deps.querier.query_wasm_smart(
                contract_addr,
                &Cw20QueryMsg::Balance {
                    address: env.contract.address.into_string(),
                },
            )?;
            balance.balance
        }
    } // deduct protocol fees
    .checked_sub(collected_protocol_fees.amount)?;

    let total_share_amount: TokenInfoResponse = deps
        .querier
        .query_wasm_smart(config.liquidity_token.clone(), &Cw20QueryMsg::TokenInfo {})?;
    let withdraw_amount =
        Decimal::from_ratio(amount, total_share_amount.total_supply) * total_asset_amount;

    // create message to send back to user if cw20
    let messages: Vec<CosmosMsg> = vec![
        match config.asset_info {
            AssetInfo::NativeToken { denom } => BankMsg::Send {
                to_address: sender.into_string(),
                amount: coins(withdraw_amount.u128(), denom),
            }
            .into(),
            AssetInfo::Token { contract_addr } => WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: sender.into_string(),
                    amount: withdraw_amount,
                })?,
                funds: vec![],
            }
            .into(),
        },
        WasmMsg::Execute {
            contract_addr: config.liquidity_token.into_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Burn { amount })?,
        }
        .into(),
    ];

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("method", "withdraw"),
        ("lp_amount", &amount.to_string()),
        ("asset_amount", &withdraw_amount.to_string()),
    ]))
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        coins,
        testing::{mock_env, mock_info},
        to_binary, Addr, BankMsg, Response, SubMsg, Uint128, WasmMsg,
    };
    use cw20::Cw20ExecuteMsg;
    use cw_multi_test::Executor;

    use pool_network::asset::{Asset, AssetInfo};
    use vault_network::vault::{Config, UpdateConfigParams};

    use crate::state::COLLECTED_PROTOCOL_FEES;
    use crate::{
        contract::execute,
        error::VaultError,
        state::CONFIG,
        tests::{
            get_fees,
            mock_app::{mock_app, mock_app_with_balance},
            mock_creator, mock_dependencies_lp, mock_execute,
            mock_instantiate::app_mock_instantiate,
            store_code::store_cw20_token_code,
        },
    };

    #[test]
    fn cannot_send_from_non_liquidity_token() {
        let (res, ..) = mock_execute(
            1,
            pool_network::asset::AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            vault_network::vault::ExecuteMsg::Receive(vault_network::vault::Cw20ReceiveMsg {
                sender: mock_creator().sender.into_string(),
                amount: Uint128::new(5_000),
                msg: to_binary(&vault_network::vault::Cw20HookMsg::Withdraw {}).unwrap(),
            }),
        );
        assert_eq!(res.unwrap_err(), VaultError::ExternalCallback {})
    }

    #[test]
    fn cannot_withdraw_when_disabled() {
        let (res, mut deps, ..) = mock_execute(
            1,
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            vault_network::vault::ExecuteMsg::UpdateConfig(UpdateConfigParams {
                flash_loan_enabled: None,
                deposit_enabled: None,
                withdraw_enabled: Some(false),
                new_owner: None,
                new_fee_collector_addr: None,
                new_vault_fees: None,
            }),
        );

        res.unwrap();

        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("", &[]),
            vault_network::vault::ExecuteMsg::Receive(vault_network::vault::Cw20ReceiveMsg {
                amount: Uint128::new(2_000),
                sender: mock_creator().sender.into_string(),
                msg: to_binary(&vault_network::vault::Cw20HookMsg::Withdraw {}).unwrap(),
            }),
        );

        assert_eq!(res.unwrap_err(), VaultError::WithdrawsDisabled {});
    }

    #[test]
    fn can_withdraw_partial_native_funds() {
        // give user 15,000 uluna to start with
        let mut app = mock_app_with_balance(vec![(mock_creator().sender, coins(15_000, "uluna"))]);

        let vault_addr = app_mock_instantiate(
            &mut app,
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
        );

        // get config for the liquidity token address
        let config: Config = app
            .wrap()
            .query_wasm_smart(
                vault_addr.clone(),
                &vault_network::vault::QueryMsg::Config {},
            )
            .unwrap();

        app.execute_contract(
            mock_creator().sender,
            vault_addr.clone(),
            &vault_network::vault::ExecuteMsg::Deposit {
                amount: Uint128::new(10_000),
            },
            &coins(10_000, "uluna"),
        )
        .unwrap();

        // withdraw 50% of funds
        app.execute_contract(
            mock_creator().sender,
            config.liquidity_token.clone(),
            &Cw20ExecuteMsg::Send {
                contract: vault_addr.to_string(),
                amount: Uint128::new(5_000),
                msg: to_binary(&vault_network::vault::Cw20HookMsg::Withdraw {}).unwrap(),
            },
            &[],
        )
        .unwrap();

        // user should now have a balance of 10_000 uluna (their 5_000 after depositing + 5_000 they just withdrew)
        assert_eq!(
            Uint128::new(10_000),
            app.wrap()
                .query_balance(mock_creator().sender, "uluna")
                .unwrap()
                .amount
        );

        // user should only have 5000 lp tokens
        let cw20_balance: cw20::BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                config.liquidity_token,
                &cw20::Cw20QueryMsg::Balance {
                    address: mock_creator().sender.into_string(),
                },
            )
            .unwrap();
        assert_eq!(Uint128::new(5_000), cw20_balance.balance)
    }

    #[test]
    fn can_withdraw_partial_token_funds() {
        let mut app = mock_app();

        // instantiate vault asset with creator having 15,000 of the asset
        let vault_asset_token_id = store_cw20_token_code(&mut app);
        let token_addr = app
            .instantiate_contract(
                vault_asset_token_id,
                mock_creator().sender,
                &cw20_base::msg::InstantiateMsg {
                    decimals: 6,
                    initial_balances: vec![cw20::Cw20Coin {
                        address: mock_creator().sender.to_string(),
                        amount: Uint128::new(15_000),
                    }],
                    marketing: None,
                    mint: None,
                    name: "CASH".to_string(),
                    symbol: "CASH".to_string(),
                },
                &[],
                "cw20_token",
                None,
            )
            .unwrap();

        let vault_asset = AssetInfo::Token {
            contract_addr: token_addr.clone().into_string(),
        };

        let vault_addr = app_mock_instantiate(&mut app, vault_asset);

        // get config for the liquidity token address
        let config: Config = app
            .wrap()
            .query_wasm_smart(
                vault_addr.clone(),
                &vault_network::vault::QueryMsg::Config {},
            )
            .unwrap();

        // increment allowance for deposit
        app.execute_contract(
            mock_creator().sender,
            token_addr.clone(),
            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                spender: vault_addr.clone().into_string(),
                amount: Uint128::new(10_000),
                expires: None,
            },
            &[],
        )
        .unwrap();

        // deposit to contract
        app.execute_contract(
            mock_creator().sender,
            vault_addr.clone(),
            &vault_network::vault::ExecuteMsg::Deposit {
                amount: Uint128::new(10_000),
            },
            &[],
        )
        .unwrap();

        // withdraw 50% of funds
        app.execute_contract(
            mock_creator().sender,
            config.liquidity_token.clone(),
            &Cw20ExecuteMsg::Send {
                contract: vault_addr.to_string(),
                amount: Uint128::new(5_000),
                msg: to_binary(&vault_network::vault::Cw20HookMsg::Withdraw {}).unwrap(),
            },
            &[],
        )
        .unwrap();

        // user should now have a balance of 10_000 token (their 5_000 left after depositing + 5_000 they just withdrew)
        let balance: cw20::BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                token_addr,
                &cw20::Cw20QueryMsg::Balance {
                    address: mock_creator().sender.into_string(),
                },
            )
            .unwrap();
        assert_eq!(Uint128::new(10_000), balance.balance);

        // user should only have 5000 lp tokens
        let cw20_balance: cw20::BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                config.liquidity_token,
                &cw20::Cw20QueryMsg::Balance {
                    address: mock_creator().sender.into_string(),
                },
            )
            .unwrap();
        assert_eq!(Uint128::new(5_000), cw20_balance.balance)
    }

    #[test]
    fn does_send_correct_response_native() {
        let env = mock_env();
        // simulate balance of 15,000 uluna in the contract
        // with two accounts, one having 10_000 of the lp token
        // and the second account just sent 5_000 of the lp token to the contract
        let mut deps = mock_dependencies_lp(
            &[(
                &env.clone().contract.address.into_string(),
                &coins(15_000, "uluna"),
            )],
            &[
                (
                    "random_acc".to_string(),
                    &[("lp_token".to_string(), Uint128::new(10_000))],
                ),
                (
                    env.clone().contract.address.into_string(),
                    &[("lp_token".to_string(), Uint128::new(4_000))],
                ),
            ],
            vec![],
        );

        // inject config
        CONFIG
            .save(
                &mut deps.storage,
                &Config {
                    liquidity_token: Addr::unchecked("lp_token"),
                    asset_info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    deposit_enabled: true,
                    flash_loan_enabled: true,
                    owner: mock_creator().sender,
                    withdraw_enabled: true,
                    fee_collector_addr: Addr::unchecked("fee_collector"),
                    fees: get_fees(),
                },
            )
            .unwrap();

        // inject protocol fees
        COLLECTED_PROTOCOL_FEES
            .save(
                &mut deps.storage,
                &Asset {
                    amount: Uint128::new(1_000),
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                },
            )
            .unwrap();

        let res = execute(
            deps.as_mut(),
            env,
            mock_info("lp_token", &[]),
            vault_network::vault::ExecuteMsg::Receive(vault_network::vault::Cw20ReceiveMsg {
                amount: Uint128::new(5_000),
                sender: mock_creator().sender.into_string(),
                msg: to_binary(&vault_network::vault::Cw20HookMsg::Withdraw {}).unwrap(),
            }),
        )
        .unwrap();

        assert_eq!(
            res,
            Response::new()
                .add_attributes(vec![
                    ("method", "withdraw"),
                    ("lp_amount", "5000"),
                    ("asset_amount", "4999"),
                ])
                .add_submessages(vec![
                    SubMsg {
                        id: 0,
                        gas_limit: None,
                        reply_on: cosmwasm_std::ReplyOn::Never,
                        msg: BankMsg::Send {
                            to_address: mock_creator().sender.into_string(),
                            amount: coins(4999, "uluna"),
                        }
                        .into(),
                    },
                    SubMsg {
                        id: 0,
                        gas_limit: None,
                        reply_on: cosmwasm_std::ReplyOn::Never,
                        msg: WasmMsg::Execute {
                            contract_addr: "lp_token".to_string(),
                            msg: to_binary(&Cw20ExecuteMsg::Burn {
                                amount: Uint128::new(5000)
                            })
                            .unwrap(),
                            funds: vec![],
                        }
                        .into(),
                    },
                ])
        );
    }

    #[test]
    fn does_send_correct_response_token() {
        let env = mock_env();
        // with two accounts, one having 10_000 of the lp token
        // and the second account just sent 5_000 of the lp token to the contract
        // contract also has 15_000 of the vault_token
        let mut deps = mock_dependencies_lp(
            &[],
            &[
                (
                    "random_acc".to_string(),
                    &[("lp_token".to_string(), Uint128::new(9_000))],
                ),
                (
                    env.clone().contract.address.into_string(),
                    &[
                        ("lp_token".to_string(), Uint128::new(5_000)),
                        ("vault_token".to_string(), Uint128::new(15_000)),
                    ],
                ),
            ],
            vec![],
        );

        // inject config
        CONFIG
            .save(
                &mut deps.storage,
                &Config {
                    liquidity_token: Addr::unchecked("lp_token"),
                    asset_info: AssetInfo::Token {
                        contract_addr: "vault_token".to_string(),
                    },
                    deposit_enabled: true,
                    flash_loan_enabled: true,
                    owner: mock_creator().sender,
                    withdraw_enabled: true,
                    fee_collector_addr: Addr::unchecked("fee_collector"),
                    fees: get_fees(),
                },
            )
            .unwrap();

        // inject protocol fees
        COLLECTED_PROTOCOL_FEES
            .save(
                &mut deps.storage,
                &Asset {
                    amount: Uint128::new(1_000),
                    info: AssetInfo::Token {
                        contract_addr: "vault_token".to_string(),
                    },
                },
            )
            .unwrap();

        let res = execute(
            deps.as_mut(),
            env,
            mock_info("lp_token", &[]),
            vault_network::vault::ExecuteMsg::Receive(vault_network::vault::Cw20ReceiveMsg {
                amount: Uint128::new(5_000),
                sender: mock_creator().sender.into_string(),
                msg: to_binary(&vault_network::vault::Cw20HookMsg::Withdraw {}).unwrap(),
            }),
        )
        .unwrap();

        assert_eq!(
            res,
            Response::new()
                .add_attributes(vec![
                    ("method", "withdraw"),
                    ("lp_amount", "5000"),
                    ("asset_amount", "4999"),
                ])
                .add_submessages(vec![
                    SubMsg {
                        id: 0,
                        gas_limit: None,
                        reply_on: cosmwasm_std::ReplyOn::Never,
                        msg: WasmMsg::Execute {
                            contract_addr: "vault_token".to_string(),
                            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                                amount: Uint128::new(4_999),
                                recipient: mock_creator().sender.into_string(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }
                        .into(),
                    },
                    SubMsg {
                        id: 0,
                        gas_limit: None,
                        reply_on: cosmwasm_std::ReplyOn::Never,
                        msg: WasmMsg::Execute {
                            contract_addr: "lp_token".to_string(),
                            msg: to_binary(&Cw20ExecuteMsg::Burn {
                                amount: Uint128::new(5000)
                            })
                            .unwrap(),
                            funds: vec![],
                        }
                        .into(),
                    },
                ])
        );
    }
}
