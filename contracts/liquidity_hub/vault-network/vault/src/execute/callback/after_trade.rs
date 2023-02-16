use cosmwasm_std::{DepsMut, Env, Response, StdError, Uint128, Uint256};
use cw20::{BalanceResponse, Cw20QueryMsg};

use pool_network::asset::{Asset, AssetInfo};

use crate::state::{store_fee, ALL_TIME_BURNED_FEES};
use crate::{
    error::VaultError,
    state::{ALL_TIME_COLLECTED_PROTOCOL_FEES, COLLECTED_PROTOCOL_FEES, CONFIG, LOAN_COUNTER},
};

pub fn after_trade(
    deps: DepsMut,
    env: Env,
    old_balance: Uint128,
    loan_amount: Uint128,
) -> Result<Response, VaultError> {
    let config = CONFIG.load(deps.storage)?;

    // query balance
    let new_balance = match config.asset_info.clone() {
        AssetInfo::NativeToken { denom } => {
            deps.querier
                .query_balance(env.contract.address.into_string(), denom)?
                .amount
        }
        AssetInfo::Token { contract_addr } => {
            let res: BalanceResponse = deps.querier.query_wasm_smart(
                contract_addr,
                &Cw20QueryMsg::Balance {
                    address: env.contract.address.into_string(),
                },
            )?;
            res.balance
        }
    };

    // check that balance is greater than expected
    let protocol_fee =
        Uint128::try_from(config.fees.protocol_fee.compute(Uint256::from(loan_amount)))?;
    let flash_loan_fee = Uint128::try_from(
        config
            .fees
            .flash_loan_fee
            .compute(Uint256::from(loan_amount)),
    )?;
    let burn_fee = Uint128::try_from(config.fees.burn_fee.compute(Uint256::from(loan_amount)))?;

    let required_amount = old_balance
        .checked_add(protocol_fee)?
        .checked_add(flash_loan_fee)?
        .checked_add(burn_fee)?;

    if required_amount > new_balance {
        return Err(VaultError::NegativeProfit {
            old_balance,
            current_balance: new_balance,
            required_amount,
        });
    }

    let profit = new_balance
        .checked_sub(old_balance)?
        .checked_sub(protocol_fee)?
        .checked_sub(flash_loan_fee)?
        .checked_sub(burn_fee)?;

    // store fees
    store_fee(deps.storage, COLLECTED_PROTOCOL_FEES, protocol_fee)?;
    store_fee(deps.storage, ALL_TIME_COLLECTED_PROTOCOL_FEES, protocol_fee)?;

    // deduct loan counter
    LOAN_COUNTER.update::<_, StdError>(deps.storage, |c| Ok(c.saturating_sub(1)))?;

    let mut response = Response::new();
    if !burn_fee.is_zero() {
        let burn_asset = Asset {
            info: config.asset_info,
            amount: burn_fee,
        };

        store_fee(deps.storage, ALL_TIME_BURNED_FEES, burn_fee)?;

        response = response.add_message(burn_asset.into_burn_msg()?);
    }

    Ok(response.add_attributes(vec![
        ("method", "after_trade".to_string()),
        ("profit", profit.to_string()),
        ("protocol_fee", protocol_fee.to_string()),
        ("flash_loan_fee", flash_loan_fee.to_string()),
        ("burn_fee", burn_fee.to_string()),
    ]))
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{
        coins,
        testing::{mock_env, mock_info},
        to_binary, Addr, BankMsg, CosmosMsg, Decimal, ReplyOn, Response, SubMsg, Uint128, WasmMsg,
    };
    use cw20::Cw20ExecuteMsg;

    use pool_network::asset::{Asset, AssetInfo};
    use vault_network::vault::Config;
    use white_whale::fee::{Fee, VaultFee};

    use crate::state::ALL_TIME_BURNED_FEES;
    use crate::{
        contract::{execute, instantiate},
        error::VaultError,
        state::{ALL_TIME_COLLECTED_PROTOCOL_FEES, COLLECTED_PROTOCOL_FEES, CONFIG, LOAN_COUNTER},
        tests::{get_fees, mock_creator, mock_dependencies_lp},
    };

    #[test]
    fn does_success_on_profit_native() {
        let env = mock_env();
        let mut deps = mock_dependencies_lp(
            &[(
                &env.clone().contract.address.into_string(),
                &coins(7_500, "uluna"),
            )],
            &[],
            vec![],
        );

        instantiate(
            deps.as_mut(),
            env.clone(),
            mock_creator(),
            vault_network::vault::InstantiateMsg {
                owner: mock_creator().sender.into_string(),
                token_id: 5,
                asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                fee_collector_addr: "fee_collector".to_string(),
                vault_fees: VaultFee {
                    flash_loan_fee: Fee {
                        share: Decimal::permille(5),
                    },
                    protocol_fee: Fee {
                        share: Decimal::permille(5),
                    },
                    burn_fee: Fee {
                        share: Decimal::permille(1),
                    },
                },
            },
        )
        .unwrap();

        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info(&env.contract.address.into_string(), &[]),
            vault_network::vault::ExecuteMsg::Callback(
                vault_network::vault::CallbackMsg::AfterTrade {
                    old_balance: Uint128::new(5_000),
                    loan_amount: Uint128::new(1_000),
                },
            ),
        )
        .unwrap();

        assert_eq!(
            res,
            Response::new()
                .add_submessage(SubMsg {
                    id: 0,
                    msg: CosmosMsg::Bank(BankMsg::Burn {
                        amount: coins(Uint128::new(1).u128(), "uluna"),
                    }),
                    gas_limit: None,
                    reply_on: ReplyOn::Never,
                })
                .add_attributes(vec![
                    ("method", "after_trade"),
                    ("profit", "2489"),
                    ("protocol_fee", "5"),
                    ("flash_loan_fee", "5"),
                    ("burn_fee", "1"),
                ])
        );

        // should have updated the protocol fee and all time fee
        let protocol_fee = COLLECTED_PROTOCOL_FEES.load(&deps.storage).unwrap();
        assert_eq!(
            protocol_fee,
            Asset {
                amount: Uint128::new(5),
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string()
                },
            }
        );
        let protocol_fee = ALL_TIME_COLLECTED_PROTOCOL_FEES
            .load(&deps.storage)
            .unwrap();
        assert_eq!(
            protocol_fee,
            Asset {
                amount: Uint128::new(5),
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string()
                },
            }
        );
    }

    #[test]
    fn does_success_on_profit_token() {
        let env = mock_env();
        let mut deps = mock_dependencies_lp(
            &[],
            &[(
                env.clone().contract.address.into_string(),
                &[("vault_token".to_string(), Uint128::new(7_500))],
            )],
            vec![],
        );

        // inject config
        CONFIG
            .save(
                &mut deps.storage,
                &Config {
                    owner: mock_creator().sender,
                    liquidity_token: Addr::unchecked("lp_token"),
                    asset_info: AssetInfo::Token {
                        contract_addr: "vault_token".to_string(),
                    },
                    deposit_enabled: true,
                    flash_loan_enabled: true,
                    withdraw_enabled: true,
                    fee_collector_addr: Addr::unchecked("fee_collector"),
                    fees: VaultFee {
                        flash_loan_fee: Fee {
                            share: Decimal::permille(5),
                        },
                        protocol_fee: Fee {
                            share: Decimal::permille(5),
                        },
                        burn_fee: Fee {
                            share: Decimal::permille(1),
                        },
                    },
                },
            )
            .unwrap();

        // inject protocol fees
        COLLECTED_PROTOCOL_FEES
            .save(
                &mut deps.storage,
                &Asset {
                    amount: Uint128::new(0),
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                },
            )
            .unwrap();
        ALL_TIME_COLLECTED_PROTOCOL_FEES
            .save(
                &mut deps.storage,
                &Asset {
                    amount: Uint128::new(0),
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                },
            )
            .unwrap();
        ALL_TIME_BURNED_FEES
            .save(
                &mut deps.storage,
                &Asset {
                    amount: Uint128::new(0),
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                },
            )
            .unwrap();

        // inject loan counter
        LOAN_COUNTER.save(&mut deps.storage, &1).unwrap();

        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info(&env.contract.address.into_string(), &[]),
            vault_network::vault::ExecuteMsg::Callback(
                vault_network::vault::CallbackMsg::AfterTrade {
                    old_balance: Uint128::new(5_000),
                    loan_amount: Uint128::new(1_000),
                },
            ),
        )
        .unwrap();

        assert_eq!(
            res,
            Response::new()
                .add_submessage(SubMsg {
                    id: 0,
                    msg: CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: "vault_token".to_string(),
                        msg: to_binary(&Cw20ExecuteMsg::Burn {
                            amount: Uint128::new(1),
                        })
                        .unwrap(),
                        funds: vec![],
                    }),
                    gas_limit: None,
                    reply_on: ReplyOn::Never,
                })
                .add_attributes(vec![
                    ("method", "after_trade"),
                    ("profit", "2489"),
                    ("protocol_fee", "5"),
                    ("flash_loan_fee", "5"),
                    ("burn_fee", "1"),
                ])
        );

        // should have updated the protocol fee and all time fee
        let protocol_fee = COLLECTED_PROTOCOL_FEES.load(&deps.storage).unwrap();
        assert_eq!(
            protocol_fee,
            Asset {
                amount: Uint128::new(5),
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string()
                },
            }
        );
        let protocol_fee = ALL_TIME_COLLECTED_PROTOCOL_FEES
            .load(&deps.storage)
            .unwrap();
        assert_eq!(
            protocol_fee,
            Asset {
                amount: Uint128::new(5),
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string()
                },
            }
        );
        let burned_fees = ALL_TIME_BURNED_FEES.load(&deps.storage).unwrap();
        assert_eq!(
            burned_fees,
            Asset {
                amount: Uint128::new(1),
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string()
                },
            }
        );
    }

    #[test]
    fn does_fail_on_negative_profit_native() {
        let env = mock_env();
        let mut deps = mock_dependencies_lp(
            &[(
                &env.clone().contract.address.into_string(),
                &coins(5_005, "uluna"),
            )],
            &[],
            vec![],
        );

        instantiate(
            deps.as_mut(),
            env.clone(),
            mock_creator(),
            vault_network::vault::InstantiateMsg {
                owner: mock_creator().sender.into_string(),
                token_id: 5,
                asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                fee_collector_addr: "fee_collector".to_string(),
                vault_fees: get_fees(),
            },
        )
        .unwrap();

        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info(&env.contract.address.into_string(), &[]),
            vault_network::vault::ExecuteMsg::Callback(
                vault_network::vault::CallbackMsg::AfterTrade {
                    old_balance: Uint128::new(5_000),
                    loan_amount: Uint128::new(1_000),
                },
            ),
        )
        .unwrap_err();

        assert_eq!(
            res,
            VaultError::NegativeProfit {
                old_balance: Uint128::new(5_000),
                current_balance: Uint128::new(5_005),
                required_amount: Uint128::new(5_010),
            }
        );
    }

    #[test]
    fn does_fail_on_negative_profit_token() {
        let env = mock_env();
        let mut deps = mock_dependencies_lp(
            &[],
            &[(
                env.clone().contract.address.into_string(),
                &[("vault_token".to_string(), Uint128::new(5_005))],
            )],
            vec![],
        );

        // inject config
        CONFIG
            .save(
                &mut deps.storage,
                &Config {
                    owner: mock_creator().sender,
                    liquidity_token: Addr::unchecked("lp_token"),
                    asset_info: AssetInfo::Token {
                        contract_addr: "vault_token".to_string(),
                    },
                    deposit_enabled: true,
                    flash_loan_enabled: true,
                    withdraw_enabled: true,
                    fee_collector_addr: Addr::unchecked("fee_collector"),
                    fees: get_fees(),
                },
            )
            .unwrap();

        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info(&env.contract.address.into_string(), &[]),
            vault_network::vault::ExecuteMsg::Callback(
                vault_network::vault::CallbackMsg::AfterTrade {
                    old_balance: Uint128::new(5_000),
                    loan_amount: Uint128::new(1_000),
                },
            ),
        )
        .unwrap_err();

        assert_eq!(
            res,
            VaultError::NegativeProfit {
                old_balance: Uint128::new(5_000),
                current_balance: Uint128::new(5_005),
                required_amount: Uint128::new(5_010),
            }
        );
    }

    #[test]
    fn does_deduct_loan_counter() {
        let env = mock_env();
        let mut deps = mock_dependencies_lp(
            &[],
            &[(
                env.clone().contract.address.into_string(),
                &[("vault_token".to_string(), Uint128::new(7_500))],
            )],
            vec![],
        );

        // inject config
        CONFIG
            .save(
                &mut deps.storage,
                &Config {
                    owner: mock_creator().sender,
                    liquidity_token: Addr::unchecked("lp_token"),
                    asset_info: AssetInfo::Token {
                        contract_addr: "vault_token".to_string(),
                    },
                    deposit_enabled: true,
                    flash_loan_enabled: true,
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
                    amount: Uint128::new(0),
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                },
            )
            .unwrap();
        ALL_TIME_COLLECTED_PROTOCOL_FEES
            .save(
                &mut deps.storage,
                &Asset {
                    amount: Uint128::new(0),
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                },
            )
            .unwrap();

        // inject loan counter
        LOAN_COUNTER.save(&mut deps.storage, &3).unwrap();

        execute(
            deps.as_mut(),
            env.clone(),
            mock_info(&env.contract.address.into_string(), &[]),
            vault_network::vault::ExecuteMsg::Callback(
                vault_network::vault::CallbackMsg::AfterTrade {
                    old_balance: Uint128::new(5_000),
                    loan_amount: Uint128::new(1_000),
                },
            ),
        )
        .unwrap();

        assert_eq!(LOAN_COUNTER.load(&deps.storage).unwrap(), 2);
    }
}
