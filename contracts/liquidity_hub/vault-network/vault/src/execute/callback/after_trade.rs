use cosmwasm_std::{DepsMut, Env, Response, Uint128};
use cw20::{BalanceResponse, Cw20QueryMsg};
use terraswap::asset::AssetInfo;

use crate::{
    error::{StdResult, VaultError},
    state::CONFIG,
};

pub fn after_trade(deps: DepsMut, env: Env, old_balance: Uint128) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    // query balance
    let new_balance = match config.asset_info {
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

    // check for profit
    if new_balance < old_balance {
        return Err(VaultError::NegativeProfit {
            old_balance,
            new_balance,
        });
    }

    let profit = new_balance.checked_sub(old_balance)?;

    Ok(Response::new().add_attributes(vec![
        ("method", "after_trade".to_string()),
        ("profit", profit.to_string()),
    ]))
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{
        coins,
        testing::{mock_env, mock_info},
        Addr, Response, Uint128,
    };
    use terraswap::asset::AssetInfo;

    use crate::{
        contract::{execute, instantiate},
        error::VaultError,
        state::{Config, CONFIG},
        tests::{mock_creator, mock_dependencies_lp},
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
                },
            ),
        )
        .unwrap();

        assert_eq!(
            res,
            Response::new().add_attributes(vec![("method", "after_trade"), ("profit", "2500")])
        );
    }

    #[test]
    fn does_success_on_profit_token() {
        let env = mock_env();
        let mut deps = mock_dependencies_lp(
            &[],
            &[(
                env.clone().contract.address.into_string(),
                &[("vault_token".to_string(), Uint128::new(7500))],
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
                },
            ),
        )
        .unwrap();

        assert_eq!(
            res,
            Response::new().add_attributes(vec![("method", "after_trade"), ("profit", "2500")])
        );
    }

    #[test]
    fn does_fail_on_negative_profit_native() {
        let env = mock_env();
        let mut deps = mock_dependencies_lp(
            &[(
                &env.clone().contract.address.into_string(),
                &coins(2_500, "uluna"),
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
                },
            ),
        )
        .unwrap_err();

        assert_eq!(
            res,
            VaultError::NegativeProfit {
                new_balance: Uint128::new(2_500),
                old_balance: Uint128::new(5_000)
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
                &[("vault_token".to_string(), Uint128::new(2_500))],
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
                },
            ),
        )
        .unwrap_err();

        assert_eq!(
            res,
            VaultError::NegativeProfit {
                new_balance: Uint128::new(2_500),
                old_balance: Uint128::new(5_000)
            }
        );
    }
}
