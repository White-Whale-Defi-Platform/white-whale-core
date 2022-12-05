use cosmwasm_std::{DepsMut, Env, Response, StdError, Uint128, Uint256};
use cw20::{BalanceResponse, Cw20QueryMsg};

use terraswap::asset::{Asset, AssetInfo};

use crate::{
    error::{StdResult, VaultError},
    state::{ALL_TIME_COLLECTED_PROTOCOL_FEES, COLLECTED_PROTOCOL_FEES, CONFIG, LOAN_COUNTER},
};

pub fn after_trade(
    deps: DepsMut,
    env: Env,
    old_balance: Uint128,
    loan_amount: Uint128,
) -> StdResult<Response> {
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

    // increment protocol fees
    COLLECTED_PROTOCOL_FEES.update::<_, StdError>(deps.storage, |mut protocol_fees| {
        protocol_fees.amount = protocol_fees.amount.checked_add(protocol_fee)?;

        Ok(protocol_fees)
    })?;
    ALL_TIME_COLLECTED_PROTOCOL_FEES.update::<_, StdError>(deps.storage, |mut protocol_fees| {
        protocol_fees.amount = protocol_fees.amount.checked_add(protocol_fee)?;

        Ok(protocol_fees)
    })?;

    // deduct loan counter
    LOAN_COUNTER.update::<_, StdError>(deps.storage, |c| Ok(c.saturating_sub(1)))?;

    let mut response = Response::new();
    if !burn_fee.is_zero() {
        let burn_asset = Asset {
            info: config.asset_info,
            amount: burn_fee,
        };

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
        Addr, Response, Uint128,
    };

    use terraswap::asset::{Asset, AssetInfo};
    use vault_network::vault::Config;

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
        .unwrap();

        assert_eq!(
            res,
            Response::new().add_attributes(vec![
                ("method", "after_trade"),
                ("profit", "2490"),
                ("protocol_fee", "5"),
                ("flash_loan_fee", "5"),
                ("burn_fee", "0"),
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
            Response::new().add_attributes(vec![
                ("method", "after_trade"),
                ("profit", "2490"),
                ("protocol_fee", "5"),
                ("flash_loan_fee", "5"),
                ("burn_fee", "0"),
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
