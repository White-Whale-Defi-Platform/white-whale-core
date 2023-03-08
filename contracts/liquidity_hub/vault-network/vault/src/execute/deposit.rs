use cosmwasm_std::{to_binary, CosmosMsg, DepsMut, Env, MessageInfo, Response, Uint128, WasmMsg};
use cw20::{AllowanceResponse, Cw20ExecuteMsg};

use pool_network::{asset::AssetInfo, querier::query_token_info};

use crate::{
    error::VaultError,
    state::{COLLECTED_PROTOCOL_FEES, CONFIG, LOAN_COUNTER},
};

pub fn deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, VaultError> {
    let config = CONFIG.load(deps.storage)?;

    // check that deposits are enabled
    if !config.deposit_enabled {
        return Err(VaultError::DepositsDisabled {});
    }

    // check that we are not currently in a flash-loan
    if LOAN_COUNTER.load(deps.storage)? != 0 {
        // more than 0 loans is being performed currently
        return Err(VaultError::DepositDuringLoan {});
    }

    // check that user sent assets they said they did
    let sent_funds = match config.asset_info.clone() {
        AssetInfo::NativeToken { denom } => info
            .funds
            .iter()
            .filter(|c| c.denom == denom)
            .map(|c| c.amount)
            .sum::<Uint128>(),
        AssetInfo::Token { contract_addr } => {
            let allowance: AllowanceResponse = deps.querier.query_wasm_smart(
                contract_addr,
                &cw20::Cw20QueryMsg::Allowance {
                    owner: info.sender.clone().into_string(),
                    spender: env.contract.address.clone().into_string(),
                },
            )?;

            allowance.allowance
        }
    };
    if sent_funds != amount {
        return Err(VaultError::FundsMismatch {
            sent: sent_funds,
            wanted: amount,
        });
    }

    let mut messages: Vec<CosmosMsg> = vec![];
    // add cw20 transfer message if needed
    if let AssetInfo::Token { contract_addr } = config.asset_info.clone() {
        messages.push(
            WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.clone().into_string(),
                    recipient: env.contract.address.clone().into_string(),
                    amount,
                })?,
                funds: vec![],
            }
            .into(),
        )
    }

    // mint LP token for the sender
    let total_lp_share =
        query_token_info(&deps.querier, config.liquidity_token.clone())?.total_supply;

    let lp_amount = if total_lp_share.is_zero() {
        // first depositor to the vault, mint LP tokens 1:1
        amount
    } else {
        // If the asset is native token, the balance has already increased in the vault
        // To calculate it properly we should subtract user deposit from the vault.
        // If the asset is a cw20 token, the balance has not changed yet so we don't need to subtract it
        let deposit_amount = match config.asset_info {
            AssetInfo::NativeToken { .. } => amount,
            AssetInfo::Token { .. } => Uint128::zero(),
        };

        // return based on a share of the total pool
        let collected_protocol_fees = COLLECTED_PROTOCOL_FEES.load(deps.storage)?;
        let total_deposits = config
            .asset_info
            .query_pool(&deps.querier, deps.api, env.contract.address)?
            .checked_sub(collected_protocol_fees.amount)?
            .checked_sub(deposit_amount)?;

        amount
            .checked_mul(total_lp_share)?
            .checked_div(total_deposits)?
    };

    messages.push(
        WasmMsg::Execute {
            contract_addr: config.liquidity_token.into_string(),
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: info.sender.into_string(),
                amount: lp_amount,
            })?,
            funds: vec![],
        }
        .into(),
    );

    Ok(Response::new()
        .add_messages(messages)
        .add_attributes(vec![("method", "deposit"), ("amount", &amount.to_string())]))
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{
        coins,
        testing::{mock_dependencies, mock_env, mock_info},
        to_binary, Addr, BankMsg, CosmosMsg, Response, Uint128, WasmMsg,
    };
    use cw20::Cw20ExecuteMsg;
    use cw_multi_test::Executor;

    use pool_network::asset::AssetInfo;
    use vault_network::vault::Config;

    use crate::tests::mock_app::mock_app_with_balance;
    use crate::tests::mock_instantiate::app_mock_instantiate;
    use crate::{
        contract::execute,
        error::VaultError,
        state::{CONFIG, LOAN_COUNTER},
        tests::{get_fees, mock_creator, mock_dependencies_lp, mock_execute},
    };

    #[test]
    fn can_deposit_native() {
        let env = mock_env();
        let mut deps = mock_dependencies_lp(
            &[],
            &[],
            vec![(
                "creator".to_string(),
                env.contract.address.clone().into_string(),
                Uint128::new(5_000),
            )],
        );

        // inject lp token address to config
        CONFIG
            .save(
                &mut deps.storage,
                &Config {
                    owner: mock_creator().sender,
                    liquidity_token: Addr::unchecked("lp_token"),
                    asset_info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    deposit_enabled: true,
                    flash_loan_enabled: true,
                    withdraw_enabled: true,
                    fee_collector_addr: Addr::unchecked("fee_collector"),
                    fees: get_fees(),
                },
            )
            .unwrap();

        // inject loan counter
        LOAN_COUNTER.save(&mut deps.storage, &0).unwrap();

        let res = execute(
            deps.as_mut(),
            env,
            mock_info("creator", &coins(5_000, "uluna")),
            vault_network::vault::ExecuteMsg::Deposit {
                amount: Uint128::new(5_000),
            },
        );

        assert_eq!(
            res.unwrap(),
            Response::new()
                .add_attributes(vec![("method", "deposit"), ("amount", "5000")])
                .add_message(WasmMsg::Execute {
                    contract_addr: "lp_token".to_string(),
                    funds: vec![],
                    msg: to_binary(&Cw20ExecuteMsg::Mint {
                        recipient: "creator".to_string(),
                        amount: Uint128::new(5_000),
                    })
                    .unwrap(),
                })
        );
    }

    #[test]
    fn can_deposit_token() {
        let env = mock_env();
        let mut deps = mock_dependencies_lp(
            &[],
            &[(
                "creator".to_string(),
                &[("vault_token".to_string(), Uint128::new(10_000))],
            )],
            vec![(
                "creator".to_string(),
                env.clone().contract.address.into_string(),
                Uint128::new(5_000),
            )],
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

        // inject loan counter
        LOAN_COUNTER.save(&mut deps.storage, &0).unwrap();

        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_creator(),
            vault_network::vault::ExecuteMsg::Deposit {
                amount: Uint128::new(5_000),
            },
        );

        assert_eq!(
            res.unwrap(),
            Response::new()
                .add_attributes(vec![("method", "deposit"), ("amount", "5000")])
                .add_messages(vec![
                    WasmMsg::Execute {
                        contract_addr: "vault_token".to_string(),
                        funds: vec![],
                        msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                            owner: "creator".to_string(),
                            recipient: env.contract.address.into_string(),
                            amount: Uint128::new(5_000),
                        })
                        .unwrap(),
                    },
                    WasmMsg::Execute {
                        contract_addr: "lp_token".to_string(),
                        funds: vec![],
                        msg: to_binary(&Cw20ExecuteMsg::Mint {
                            recipient: "creator".to_string(),
                            amount: Uint128::new(5_000),
                        })
                        .unwrap(),
                    },
                ])
        )
    }

    #[test]
    fn does_verify_funds_deposited_native() {
        let (res, ..) = mock_execute(
            2,
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            vault_network::vault::ExecuteMsg::Deposit {
                amount: Uint128::new(5_000),
            },
        );

        assert_eq!(
            res.unwrap_err(),
            VaultError::FundsMismatch {
                sent: Uint128::new(0),
                wanted: Uint128::new(5_000),
            }
        );
    }

    #[test]
    fn does_verify_funds_deposited_token() {
        let env = mock_env();
        let mut deps = mock_dependencies_lp(&[], &[], vec![]);

        // inject config
        CONFIG
            .save(
                &mut deps.storage,
                &Config {
                    owner: mock_creator().sender,
                    asset_info: AssetInfo::Token {
                        contract_addr: "vault_token".to_string(),
                    },
                    liquidity_token: Addr::unchecked("lp_token"),
                    deposit_enabled: true,
                    flash_loan_enabled: true,
                    withdraw_enabled: true,
                    fee_collector_addr: Addr::unchecked("fee_collector"),
                    fees: get_fees(),
                },
            )
            .unwrap();

        // inject loan counter
        LOAN_COUNTER.save(&mut deps.storage, &0).unwrap();

        let res = execute(
            deps.as_mut(),
            env,
            mock_creator(),
            vault_network::vault::ExecuteMsg::Deposit {
                amount: Uint128::new(5_000),
            },
        );

        assert_eq!(
            res.unwrap_err(),
            VaultError::FundsMismatch {
                sent: Uint128::new(0),
                wanted: Uint128::new(5_000),
            }
        );
    }

    #[test]
    fn cannot_deposit_when_disabled() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        // inject config
        CONFIG
            .save(
                &mut deps.storage,
                &Config {
                    owner: mock_creator().sender,
                    asset_info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    liquidity_token: Addr::unchecked("lp_token"),
                    deposit_enabled: false,
                    flash_loan_enabled: true,
                    withdraw_enabled: true,
                    fee_collector_addr: Addr::unchecked("fee_collector_addr"),
                    fees: get_fees(),
                },
            )
            .unwrap();

        let res = execute(
            deps.as_mut(),
            env,
            mock_creator(),
            vault_network::vault::ExecuteMsg::Deposit {
                amount: Uint128::new(5_000),
            },
        );

        assert_eq!(res.unwrap_err(), VaultError::DepositsDisabled {});
    }

    #[test]
    fn cannot_deposit_when_loan() {
        let mut deps = mock_dependencies();

        // inject config
        CONFIG
            .save(
                &mut deps.storage,
                &Config {
                    owner: mock_creator().sender,
                    asset_info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    liquidity_token: Addr::unchecked("lp_token"),
                    deposit_enabled: true,
                    flash_loan_enabled: true,
                    withdraw_enabled: true,
                    fee_collector_addr: Addr::unchecked("fee_collector_addr"),
                    fees: get_fees(),
                },
            )
            .unwrap();

        // inject loan state
        LOAN_COUNTER.save(&mut deps.storage, &2).unwrap();

        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_creator(),
            vault_network::vault::ExecuteMsg::Deposit {
                amount: Uint128::new(5_000),
            },
        );

        assert_eq!(res.unwrap_err(), VaultError::DepositDuringLoan {});
    }

    #[test]
    fn does_not_dilute_early_holders() {
        // simulate a vault with first depositor having 10,000 LP tokens
        // and vault having 15,000 of asset
        // the next depositor should not deposit at a 1:1 rate for asset:LP tokens
        // otherwise, the earlier depositor will be diluted.
        let second_depositor = Addr::unchecked("depositor2");
        let third_depositor = Addr::unchecked("depositor3");

        let mut app = mock_app_with_balance(vec![
            (mock_creator().sender, coins(15_000, "uluna")),
            (second_depositor.clone(), coins(5_000, "uluna")),
            (third_depositor.clone(), coins(8_000, "uluna")),
        ]);

        let vault_addr = app_mock_instantiate(
            &mut app,
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
        );

        // first depositor deposits 10,000 uluna
        app.execute_contract(
            mock_creator().sender,
            vault_addr.clone(),
            &vault_network::vault::ExecuteMsg::Deposit {
                amount: Uint128::new(10_000),
            },
            &coins(10_000, "uluna"),
        )
        .unwrap();

        // get config for the liquidity token address
        let config: Config = app
            .wrap()
            .query_wasm_smart(
                vault_addr.clone(),
                &vault_network::vault::QueryMsg::Config {},
            )
            .unwrap();

        // user should have 10,000 lp tokens
        let cw20_balance: cw20::BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                config.liquidity_token.clone(),
                &cw20::Cw20QueryMsg::Balance {
                    address: mock_creator().sender.into_string(),
                },
            )
            .unwrap();
        assert_eq!(Uint128::new(10_000), cw20_balance.balance);

        // inject 5,000 luna that where "generated" via fees
        app.execute(
            mock_creator().sender,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: vault_addr.to_string(),
                amount: coins(5000, "uluna"),
            }),
        )
        .unwrap();

        // second depositor deposits 5,000 uluna
        app.execute_contract(
            second_depositor.clone(),
            vault_addr.clone(),
            &vault_network::vault::ExecuteMsg::Deposit {
                amount: Uint128::new(5_000),
            },
            &coins(5_000, "uluna"),
        )
        .unwrap();

        // creator has 10,000 LP tokens in a 15,000 uluna pool
        // depositor2 should therefore get (5000 / 15000) * 10000 = 3,333 LP tokens
        let cw20_balance: cw20::BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                config.liquidity_token.clone(),
                &cw20::Cw20QueryMsg::Balance {
                    address: second_depositor.to_string(),
                },
            )
            .unwrap();
        assert_eq!(Uint128::new(3_333), cw20_balance.balance);

        // third depositor deposits 8,000 uluna
        app.execute_contract(
            third_depositor.clone(),
            vault_addr,
            &vault_network::vault::ExecuteMsg::Deposit {
                amount: Uint128::new(8_000),
            },
            &coins(8_000, "uluna"),
        )
        .unwrap();

        // creator has 10,000 LP tokens in a 20,000 uluna pool
        // depositor2 has 3,333 LP tokens
        // depositor3 should therefore get (8000 / 20000) * 13333 = 5,333 LP tokens
        let cw20_balance: cw20::BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                config.liquidity_token,
                &cw20::Cw20QueryMsg::Balance {
                    address: third_depositor.to_string(),
                },
            )
            .unwrap();
        assert_eq!(Uint128::new(5_333), cw20_balance.balance);

        // at the point the pool has 28,000 uluna for a total of 18,666 LP tokens
        // this leaves creator with 10,000 / 18,666 of the total LP supply or 15,000 tokens
        // depositor2 is entitled to 3,333 / 18,666 of the total LP supply or 5,000 tokens
        // depositor3 is entitled to 5,333 / 18,666 of the total LP supply or 8,000 tokens
    }
}
