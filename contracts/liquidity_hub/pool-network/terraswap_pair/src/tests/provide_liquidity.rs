use crate::contract::{execute, instantiate, reply};
use crate::error::ContractError;
use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    to_binary, Coin, CosmosMsg, Decimal, Reply, Response, StdError, SubMsg, SubMsgResponse,
    SubMsgResult, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use pool_network::asset::{Asset, AssetInfo, PairType, MINIMUM_LIQUIDITY_AMOUNT};
use pool_network::mock_querier::mock_dependencies;
use pool_network::pair::{ExecuteMsg, InstantiateMsg, PoolFee};
use white_whale::fee::Fee;

#[test]
fn provide_liquidity() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(2_000u128),
    }]);

    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::zero())],
        ),
        (&"asset0000".to_string(), &[]),
    ]);

    let msg = InstantiateMsg {
        asset_infos: [
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            AssetInfo::Token {
                contract_addr: "asset0000".to_string(),
            },
        ],
        token_code_id: 10u64,
        asset_decimals: [6u8, 8u8],
        pool_fees: PoolFee {
            protocol_fee: Fee {
                share: Decimal::percent(1u64),
            },
            swap_fee: Fee {
                share: Decimal::percent(1u64),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
        },
        fee_collector_addr: "collector".to_string(),
        pair_type: PairType::ConstantProduct,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    // store liquidity token
    let reply_msg = Reply {
        id: 1,
        result: SubMsgResult::Ok(SubMsgResponse {
            events: vec![],
            data: Some(
                vec![
                    10, 13, 108, 105, 113, 117, 105, 100, 105, 116, 121, 48, 48, 48, 48,
                ]
                .into(),
            ),
        }),
    };

    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    // unsuccessfully providing liquidity since share becomes zero, MINIMUM_LIQUIDITY_AMOUNT provided
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: "asset0000".to_string(),
                },
                amount: MINIMUM_LIQUIDITY_AMOUNT,
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: MINIMUM_LIQUIDITY_AMOUNT,
            },
        ],
        slippage_tolerance: None,
        receiver: None,
    };

    let env = mock_env();
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: MINIMUM_LIQUIDITY_AMOUNT,
        }],
    );
    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    match res {
        ContractError::InvalidInitialLiquidityAmount { .. } => {}
        _ => panic!("should return ContractError::InvalidInitialLiquidityAmount"),
    }

    // successfully provide liquidity for the exist pool
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: "asset0000".to_string(),
                },
                amount: Uint128::from(2_000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(2_000u128),
            },
        ],
        slippage_tolerance: None,
        receiver: None,
    };

    let env = mock_env();
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(2_000u128),
        }],
    );
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(res.messages.len(), 3usize);

    let transfer_from_msg = res.messages.get(0).expect("no message");
    let mint_initial_lp_msg = res.messages.get(1).expect("no message");
    let mint_msg = res.messages.get(2).expect("no message");
    assert_eq!(
        transfer_from_msg,
        &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset0000".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                owner: "addr0000".to_string(),
                recipient: MOCK_CONTRACT_ADDR.to_string(),
                amount: Uint128::from(2_000u128),
            })
            .unwrap(),
            funds: vec![],
        }))
    );
    assert_eq!(
        mint_initial_lp_msg,
        &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "liquidity0000".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: "cosmos2contract".to_string(),
                amount: MINIMUM_LIQUIDITY_AMOUNT,
            })
            .unwrap(),
            funds: vec![],
        }))
    );
    assert_eq!(
        mint_msg,
        &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "liquidity0000".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: "addr0000".to_string(),
                amount: Uint128::from(1_000u128),
            })
            .unwrap(),
            funds: vec![],
        }))
    );

    // provide more liquidity 1:2, which is not proportional to 1:1,
    // then it must accept 1:1 and treat left amount as donation
    deps.querier.with_balance(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(
                200u128 + 200u128, /* user deposit must be pre-applied */
            ),
        }],
    )]);

    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(100u128))],
        ),
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(200u128))],
        ),
    ]);

    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: "asset0000".to_string(),
                },
                amount: Uint128::from(100u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(200u128),
            },
        ],
        slippage_tolerance: None,
        receiver: Some("staking0000".to_string()), // try changing receiver
    };

    let env = mock_env();
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(200u128),
        }],
    );

    // only accept 100, then 50 share will be generated with 100 * (100 / 200)
    let res: Response = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(res.messages.len(), 2usize);

    let transfer_from_msg = res.messages.get(0).expect("no message");
    let mint_msg = res.messages.get(1).expect("no message");
    assert_eq!(
        transfer_from_msg,
        &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset0000".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                owner: "addr0000".to_string(),
                recipient: MOCK_CONTRACT_ADDR.to_string(),
                amount: Uint128::from(100u128),
            })
            .unwrap(),
            funds: vec![],
        }))
    );
    assert_eq!(
        mint_msg,
        &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "liquidity0000".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: "staking0000".to_string(), // LP tokens sent to specified receiver
                amount: Uint128::from(50u128),
            })
            .unwrap(),
            funds: vec![],
        }))
    );

    // check wrong argument
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: "asset0000".to_string(),
                },
                amount: Uint128::from(100u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(50u128),
            },
        ],
        slippage_tolerance: None,
        receiver: None,
    };

    let env = mock_env();
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );
    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    match res {
        ContractError::Std(StdError::GenericErr { msg, .. }) => assert_eq!(
            msg,
            "Native token balance mismatch between the argument and the transferred".to_string()
        ),
        _ => panic!("Must return generic error"),
    }

    // initialize token balance to 1:1
    deps.querier.with_balance(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(
                100u128 + 100u128, /* user deposit must be pre-applied */
            ),
        }],
    )]);

    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(100u128))],
        ),
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(100u128))],
        ),
    ]);

    // failed because the price is under slippage_tolerance
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: "asset0000".to_string(),
                },
                amount: Uint128::from(98u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(100u128),
            },
        ],
        slippage_tolerance: Some(Decimal::percent(1)),
        receiver: None,
    };

    let env = mock_env();
    let info = mock_info(
        "addr0001",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );
    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    match res {
        ContractError::MaxSlippageAssertion {} => {}
        _ => panic!("DO NOT ENTER HERE"),
    }

    // initialize token balance to 1:1
    deps.querier.with_balance(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128 + 98u128 /* user deposit must be pre-applied */),
        }],
    )]);

    // failed because the price is under slippage_tolerance
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: "asset0000".to_string(),
                },
                amount: Uint128::from(100u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(98u128),
            },
        ],
        slippage_tolerance: Some(Decimal::percent(1)),
        receiver: None,
    };

    let env = mock_env();
    let info = mock_info(
        "addr0001",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(98u128),
        }],
    );
    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    match res {
        ContractError::MaxSlippageAssertion {} => {}
        _ => panic!("DO NOT ENTER HERE"),
    }

    // initialize token balance to 1:1
    deps.querier.with_balance(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(
                100u128 + 100u128, /* user deposit must be pre-applied */
            ),
        }],
    )]);

    // successfully provides
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: "asset0000".to_string(),
                },
                amount: Uint128::from(99u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(100u128),
            },
        ],
        slippage_tolerance: Some(Decimal::percent(1)),
        receiver: None,
    };

    let env = mock_env();
    let info = mock_info(
        "addr0001",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );
    let _res = execute(deps.as_mut(), env, info, msg).unwrap();

    // initialize token balance to 1:1
    deps.querier.with_balance(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128 + 99u128 /* user deposit must be pre-applied */),
        }],
    )]);

    // successfully provides
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: "asset0000".to_string(),
                },
                amount: Uint128::from(100u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(99u128),
            },
        ],
        slippage_tolerance: Some(Decimal::percent(1)),
        receiver: None,
    };

    let env = mock_env();
    let info = mock_info(
        "addr0001",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(99u128),
        }],
    );
    let _res = execute(deps.as_mut(), env, info, msg).unwrap();
}

#[test]
fn provide_liquidity_zero_amount() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(200u128),
    }]);

    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::zero())],
        ),
        (&"asset0000".to_string(), &[]),
    ]);

    let msg = InstantiateMsg {
        asset_infos: [
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            AssetInfo::Token {
                contract_addr: "asset0000".to_string(),
            },
        ],
        token_code_id: 10u64,
        asset_decimals: [6u8, 8u8],
        pool_fees: PoolFee {
            protocol_fee: Fee {
                share: Decimal::percent(1u64),
            },
            swap_fee: Fee {
                share: Decimal::percent(1u64),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
        },
        fee_collector_addr: "collector".to_string(),
        pair_type: PairType::ConstantProduct,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    // store liquidity token
    let reply_msg = Reply {
        id: 1,
        result: SubMsgResult::Ok(SubMsgResponse {
            events: vec![],
            data: Some(
                vec![
                    10, 13, 108, 105, 113, 117, 105, 100, 105, 116, 121, 48, 48, 48, 48,
                ]
                .into(),
            ),
        }),
    };

    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    // provide invalid (zero) liquidity
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: "asset0000".to_string(),
                },
                amount: Uint128::zero(),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(100u128),
            },
        ],
        slippage_tolerance: None,
        receiver: None,
    };

    let env = mock_env();
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );
    let res = execute(deps.as_mut(), env, info, msg);

    match res {
        Ok(_) => panic!("should return ContractError::InvalidZeroAmount"),
        Err(ContractError::InvalidZeroAmount {}) => {}
        _ => panic!("should return ContractError::InvalidZeroAmount"),
    }
}

#[test]
fn provide_liquidity_invalid_minimum_lp_amount() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(200u128),
    }]);

    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::zero())],
        ),
        (&"asset0000".to_string(), &[]),
    ]);

    let msg = InstantiateMsg {
        asset_infos: [
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            AssetInfo::Token {
                contract_addr: "asset0000".to_string(),
            },
        ],
        token_code_id: 10u64,
        asset_decimals: [6u8, 8u8],
        pool_fees: PoolFee {
            protocol_fee: Fee {
                share: Decimal::percent(1u64),
            },
            swap_fee: Fee {
                share: Decimal::percent(1u64),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
        },
        fee_collector_addr: "collector".to_string(),
        pair_type: PairType::ConstantProduct,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    // store liquidity token
    let reply_msg = Reply {
        id: 1,
        result: SubMsgResult::Ok(SubMsgResponse {
            events: vec![],
            data: Some(
                vec![
                    10, 13, 108, 105, 113, 117, 105, 100, 105, 116, 121, 48, 48, 48, 48,
                ]
                .into(),
            ),
        }),
    };

    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    // successfully provide liquidity for the exist pool
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: "asset0000".to_string(),
                },
                amount: (MINIMUM_LIQUIDITY_AMOUNT - Uint128::one()),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(100u128),
            },
        ],
        slippage_tolerance: None,
        receiver: None,
    };

    let env = mock_env();
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );
    let res = execute(deps.as_mut(), env, info, msg);

    match res {
        Ok(_) => panic!("should return ContractError::InvalidInitialLiquidityAmount"),
        Err(ContractError::InvalidInitialLiquidityAmount { .. }) => {}
        _ => panic!("should return ContractError::InvalidInitialLiquidityAmount"),
    }
}
