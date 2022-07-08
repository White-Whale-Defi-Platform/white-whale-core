use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    attr, to_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal, Reply, ReplyOn, Response, StdError,
    SubMsg, SubMsgResponse, SubMsgResult, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg, MinterResponse};

use terraswap::asset::{Asset, AssetInfo, PairInfo};
use terraswap::mock_querier::mock_dependencies;
use terraswap::pair::ExecuteMsg::UpdateConfig;
use terraswap::pair::{
    Cw20HookMsg, ExecuteMsg, FeatureToggle, InstantiateMsg, PoolFee, PoolResponse,
    ReverseSimulationResponse, SimulationResponse,
};
use terraswap::token::InstantiateMsg as TokenInstantiateMsg;
use white_whale::fee::Fee;

use crate::contract::{execute, instantiate, reply};
use crate::error::ContractError;
use crate::helpers::{assert_max_spread, compute_swap};
use crate::queries::{
    query_config, query_pair_info, query_pool, query_protocol_fees, query_reverse_simulation,
    query_simulation,
};
use crate::state::{get_protocol_fees_for_asset, store_protocol_fee, COLLECTED_PROTOCOL_FEES};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    deps.querier.with_token_balances(&[(
        &"asset0000".to_string(),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(123u128))],
    )]);

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
        },
    };

    // we can just call .unwrap() to assert this was a success
    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg {
            msg: WasmMsg::Instantiate {
                code_id: 10u64,
                msg: to_binary(&TokenInstantiateMsg {
                    name: "uusd-mAAPL-LP".to_string(),
                    symbol: "uLP".to_string(),
                    decimals: 6,
                    initial_balances: vec![],
                    mint: Some(MinterResponse {
                        minter: MOCK_CONTRACT_ADDR.to_string(),
                        cap: None,
                    }),
                })
                .unwrap(),
                funds: vec![],
                label: "uusd-mAAPL-LP".to_string(),
                admin: None,
            }
            .into(),
            gas_limit: None,
            id: 1,
            reply_on: ReplyOn::Success,
        }]
    );

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

    // it worked, let's query the state
    let pair_info: PairInfo = query_pair_info(deps.as_ref()).unwrap();
    assert_eq!("liquidity0000", pair_info.liquidity_token.as_str());
    assert_eq!(
        pair_info.asset_infos,
        [
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            AssetInfo::Token {
                contract_addr: "asset0000".to_string()
            }
        ]
    );
}

#[test]
fn test_initialization_invalid_fees() {
    let mut deps = mock_dependencies(&[]);

    deps.querier.with_token_balances(&[(
        &"asset0000".to_string(),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(123u128))],
    )]);

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
                share: Decimal::from_ratio(Uint128::from(2u8), Uint128::from(1u8)),
            },
            swap_fee: Fee {
                share: Decimal::percent(1u64),
            },
        },
    };

    // we can just call .unwrap() to assert this was a success
    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let res = instantiate(deps.as_mut(), env, info, msg);
    match res {
        Ok(_) => panic!("should return StdError::generic_err(Invalid fee)"),
        Err(StdError::GenericErr { .. }) => (),
        _ => panic!("should return StdError::generic_err(Invalid fee)"),
    }
}

#[test]
fn provide_liquidity() {
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
        },
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
                amount: Uint128::from(100u128),
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
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
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
                recipient: "addr0000".to_string(),
                amount: Uint128::from(100u128),
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
fn withdraw_liquidity() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(100u128),
    }]);

    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&"addr0000".to_string(), &Uint128::from(100u128))],
        ),
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(100u128))],
        ),
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
        },
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

    // store some protocol fees in both native and token
    store_protocol_fee(
        deps.as_mut().storage,
        Uint128::from(10u8),
        "uusd".to_string(),
        COLLECTED_PROTOCOL_FEES,
    )
    .unwrap();
    store_protocol_fee(
        deps.as_mut().storage,
        Uint128::from(20u8),
        "asset0000".to_string(),
        COLLECTED_PROTOCOL_FEES,
    )
    .unwrap();

    // withdraw liquidity
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        msg: to_binary(&Cw20HookMsg::WithdrawLiquidity {}).unwrap(),
        amount: Uint128::from(100u128),
    });

    let env = mock_env();
    let info = mock_info("liquidity0000", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    let log_withdrawn_share = res.attributes.get(2).expect("no log");
    let log_refund_assets = res.attributes.get(3).expect("no log");
    let msg_refund_0 = res.messages.get(0).expect("no message");
    let msg_refund_1 = res.messages.get(1).expect("no message");
    let msg_burn_liquidity = res.messages.get(2).expect("no message");

    let protocol_fee_native =
        get_protocol_fees_for_asset(deps.as_mut().storage, "uusd".to_string()).unwrap();
    let expected_native_refund_amount: Uint128 = Uint128::from(100u128)
        .checked_sub(protocol_fee_native.amount)
        .unwrap();

    let protocol_fee_token =
        get_protocol_fees_for_asset(deps.as_mut().storage, "asset0000".to_string()).unwrap();
    let expected_token_refund_amount: Uint128 = Uint128::from(100u128)
        .checked_sub(protocol_fee_token.amount)
        .unwrap();

    assert_eq!(
        msg_refund_0,
        &SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "addr0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: expected_native_refund_amount,
            }],
        }))
    );
    assert_eq!(
        msg_refund_1,
        &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset0000".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "addr0000".to_string(),
                amount: expected_token_refund_amount,
            })
            .unwrap(),
            funds: vec![],
        }))
    );
    assert_eq!(
        msg_burn_liquidity,
        &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "liquidity0000".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Burn {
                amount: Uint128::from(100u128),
            })
            .unwrap(),
            funds: vec![],
        }))
    );

    assert_eq!(
        log_withdrawn_share,
        &attr("withdrawn_share", 100u128.to_string())
    );
    assert_eq!(
        log_refund_assets,
        &attr("refund_assets", "90uusd, 80asset0000")
    );
}

#[test]
fn try_native_to_token() {
    let total_share = Uint128::from(30000000000u128);
    let asset_pool_amount = Uint128::from(20000000000u128);
    let collateral_pool_amount = Uint128::from(30000000000u128);
    let exchange_rate: Decimal = Decimal::from_ratio(asset_pool_amount, collateral_pool_amount);
    let offer_amount = Uint128::from(1500000000u128);

    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: collateral_pool_amount + offer_amount,
        /* user deposit must be pre-applied */
    }]);

    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &total_share)],
        ),
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &asset_pool_amount)],
        ),
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
                share: Decimal::from_ratio(1u128, 1000u128),
            },
            swap_fee: Fee {
                share: Decimal::from_ratio(3u128, 1000u128),
            },
        },
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

    // normal swap
    let msg = ExecuteMsg::Swap {
        offer_asset: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: offer_amount,
        },
        belief_price: None,
        max_spread: None,
        to: None,
    };
    let env = mock_env();
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: offer_amount,
        }],
    );
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    let msg_transfer = res.messages.get(0).expect("no message");

    // current price is 1.5, so expected return without spread is 1000
    // ask_amount = (ask_pool * offer_amount / (offer_pool + offer_amount))
    // 952.380952 = 20000 * 1500 / (30000 + 1500) - swap_fee - protocol_fee
    let expected_ret_amount = Uint128::from(952_380_952u128);
    let expected_spread_amount = (offer_amount * exchange_rate)
        .checked_sub(expected_ret_amount)
        .unwrap();
    let expected_swap_fee_amount = expected_ret_amount.multiply_ratio(3u128, 1000u128); // 0.3%
    let expected_protocol_fee_amount = expected_ret_amount.multiply_ratio(1u128, 1000u128); // 0.1%
    let expected_return_amount = expected_ret_amount
        .checked_sub(expected_swap_fee_amount)
        .unwrap()
        .checked_sub(expected_protocol_fee_amount)
        .unwrap();
    // check simulation res
    deps.querier.with_balance(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: collateral_pool_amount,
            /* user deposit must be pre-applied */
        }],
    )]);

    let simulation_res: SimulationResponse = query_simulation(
        deps.as_ref(),
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: offer_amount,
        },
    )
    .unwrap();
    assert_eq!(expected_return_amount, simulation_res.return_amount);
    assert_eq!(expected_swap_fee_amount, simulation_res.swap_fee_amount);
    assert_eq!(expected_spread_amount, simulation_res.spread_amount);
    assert_eq!(
        expected_protocol_fee_amount,
        simulation_res.protocol_fee_amount
    );

    // as we swapped native to token, we accumulate the protocol fees in token
    let protocol_fees_for_token =
        query_protocol_fees(deps.as_ref(), Some("asset0000".to_string()), None)
            .unwrap()
            .fees;
    assert_eq!(
        protocol_fees_for_token.first().unwrap().amount,
        expected_protocol_fee_amount
    );
    let protocol_fees_for_native =
        query_protocol_fees(deps.as_ref(), Some("uusd".to_string()), None)
            .unwrap()
            .fees;
    assert_eq!(
        protocol_fees_for_native.first().unwrap().amount,
        Uint128::zero()
    );

    // check reverse simulation res
    let reverse_simulation_res: ReverseSimulationResponse = query_reverse_simulation(
        deps.as_ref(),
        Asset {
            info: AssetInfo::Token {
                contract_addr: "asset0000".to_string(),
            },
            amount: expected_return_amount,
        },
    )
    .unwrap();

    assert!(
        (offer_amount.u128() as i128 - reverse_simulation_res.offer_amount.u128() as i128).abs()
            < 3i128
    );
    assert!(
        (expected_swap_fee_amount.u128() as i128
            - reverse_simulation_res.swap_fee_amount.u128() as i128)
            .abs()
            < 3i128
    );
    assert!(
        (expected_spread_amount.u128() as i128
            - reverse_simulation_res.spread_amount.u128() as i128)
            .abs()
            < 3i128
    );
    assert!(
        (expected_protocol_fee_amount.u128() as i128
            - reverse_simulation_res.protocol_fee_amount.u128() as i128)
            .abs()
            < 3i128
    );

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "swap"),
            attr("sender", "addr0000"),
            attr("receiver", "addr0000"),
            attr("offer_asset", "uusd"),
            attr("ask_asset", "asset0000"),
            attr("offer_amount", offer_amount.to_string()),
            attr("return_amount", expected_return_amount.to_string()),
            attr("spread_amount", expected_spread_amount.to_string()),
            attr("swap_fee_amount", expected_swap_fee_amount.to_string()),
            attr(
                "protocol_fee_amount",
                expected_protocol_fee_amount.to_string(),
            ),
        ]
    );

    assert_eq!(
        &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset0000".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "addr0000".to_string(),
                amount: expected_return_amount,
            })
            .unwrap(),
            funds: vec![],
        })),
        msg_transfer,
    );
}

#[test]
fn try_token_to_native() {
    let total_share = Uint128::from(20_000_000_000u128);
    let asset_pool_amount = Uint128::from(30_000_000_000u128);
    let collateral_pool_amount = Uint128::from(20_000_000_000u128);
    let exchange_rate = Decimal::from_ratio(collateral_pool_amount, asset_pool_amount);
    let offer_amount = Uint128::from(1_500_000_000u128);

    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: collateral_pool_amount,
    }]);
    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &total_share)],
        ),
        (
            &"asset0000".to_string(),
            &[(
                &MOCK_CONTRACT_ADDR.to_string(),
                &(asset_pool_amount + offer_amount),
            )],
        ),
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
        asset_decimals: [8u8, 8u8],
        pool_fees: PoolFee {
            protocol_fee: Fee {
                share: Decimal::from_ratio(1u128, 1000u128),
            },
            swap_fee: Fee {
                share: Decimal::from_ratio(3u128, 1000u128),
            },
        },
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

    // unauthorized access; can not execute swap directly for token swap
    let msg = ExecuteMsg::Swap {
        offer_asset: Asset {
            info: AssetInfo::Token {
                contract_addr: "asset0000".to_string(),
            },
            amount: offer_amount,
        },
        belief_price: None,
        max_spread: None,
        to: None,
    };
    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    match res {
        ContractError::Unauthorized {} => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    // normal sell
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        amount: offer_amount,
        msg: to_binary(&Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: None,
            to: None,
        })
        .unwrap(),
    });
    let env = mock_env();
    let info = mock_info("asset0000", &[]);

    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    let msg_transfer = res.messages.get(0).expect("no message");

    // current price is 1.5, so expected return without spread is 1000
    // ask_amount = (ask_pool * offer_amount / (offer_pool + offer_amount))
    // 952.380952 = 20000 * 1500 / (30000 + 1500) - swap_fee - protocol_fee
    let expected_ret_amount = Uint128::from(952_380_952u128);
    let expected_spread_amount = (offer_amount * exchange_rate)
        .checked_sub(expected_ret_amount)
        .unwrap();
    let expected_swap_fee_amount = expected_ret_amount.multiply_ratio(3u128, 1000u128); // 0.3%
    let expected_protocol_fee_amount = expected_ret_amount.multiply_ratio(1u128, 1000u128); // 0.1%
    let expected_return_amount = expected_ret_amount
        .checked_sub(expected_swap_fee_amount)
        .unwrap()
        .checked_sub(expected_protocol_fee_amount)
        .unwrap();

    // check simulation res
    // return asset token balance as normal
    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &total_share)],
        ),
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &(asset_pool_amount))],
        ),
    ]);

    let simulation_res: SimulationResponse = query_simulation(
        deps.as_ref(),
        Asset {
            amount: offer_amount,
            info: AssetInfo::Token {
                contract_addr: "asset0000".to_string(),
            },
        },
    )
    .unwrap();

    assert_eq!(expected_return_amount, simulation_res.return_amount);
    assert_eq!(expected_swap_fee_amount, simulation_res.swap_fee_amount);
    assert_eq!(expected_spread_amount, simulation_res.spread_amount);
    assert_eq!(
        expected_protocol_fee_amount,
        simulation_res.protocol_fee_amount
    );

    // as we swapped token to native, we accumulate the protocol fees in native
    let protocol_fees_for_native =
        query_protocol_fees(deps.as_ref(), Some("uusd".to_string()), None)
            .unwrap()
            .fees;
    assert_eq!(
        protocol_fees_for_native.first().unwrap().amount,
        expected_protocol_fee_amount
    );
    let protocol_fees_for_token =
        query_protocol_fees(deps.as_ref(), Some("asset0000".to_string()), None)
            .unwrap()
            .fees;
    assert_eq!(
        protocol_fees_for_token.first().unwrap().amount,
        Uint128::zero()
    );

    // check reverse simulation res
    let reverse_simulation_res: ReverseSimulationResponse = query_reverse_simulation(
        deps.as_ref(),
        Asset {
            amount: expected_return_amount,
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
        },
    )
    .unwrap();

    assert!(
        (offer_amount.u128() as i128 - reverse_simulation_res.offer_amount.u128() as i128).abs()
            < 3i128
    );
    assert!(
        (expected_swap_fee_amount.u128() as i128
            - reverse_simulation_res.swap_fee_amount.u128() as i128)
            .abs()
            < 3i128
    );
    assert!(
        (expected_spread_amount.u128() as i128
            - reverse_simulation_res.spread_amount.u128() as i128)
            .abs()
            < 3i128
    );

    assert!(
        (expected_protocol_fee_amount.u128() as i128
            - reverse_simulation_res.protocol_fee_amount.u128() as i128)
            .abs()
            < 3i128
    );

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "swap"),
            attr("sender", "addr0000"),
            attr("receiver", "addr0000"),
            attr("offer_asset", "asset0000"),
            attr("ask_asset", "uusd"),
            attr("offer_amount", offer_amount.to_string()),
            attr("return_amount", expected_return_amount.to_string()),
            attr("spread_amount", expected_spread_amount.to_string()),
            attr("swap_fee_amount", expected_swap_fee_amount.to_string()),
            attr(
                "protocol_fee_amount",
                expected_protocol_fee_amount.to_string(),
            ),
        ]
    );

    assert_eq!(
        &SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "addr0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: expected_return_amount,
            }],
        })),
        msg_transfer,
    );

    // failed due to non asset token contract try to execute sell
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        amount: offer_amount,
        msg: to_binary(&Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: None,
            to: None,
        })
        .unwrap(),
    });
    let env = mock_env();
    let info = mock_info("liquidity0000", &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    match res {
        ContractError::Unauthorized {} => (),
        _ => panic!("DO NOT ENTER HERE"),
    }
}

#[test]
fn test_max_spread() {
    let offer_asset_info = AssetInfo::NativeToken {
        denom: "offer_asset".to_string(),
    };
    let ask_asset_info = AssetInfo::NativeToken {
        denom: "ask_asset_info".to_string(),
    };

    assert_max_spread(
        Some(Decimal::from_ratio(1200u128, 1u128)),
        Some(Decimal::percent(1)),
        Asset {
            info: offer_asset_info.clone(),
            amount: Uint128::from(1200000000u128),
        },
        Asset {
            info: ask_asset_info.clone(),
            amount: Uint128::from(989999u128),
        },
        Uint128::zero(),
        6u8,
        6u8,
    )
    .unwrap_err();

    assert_max_spread(
        Some(Decimal::from_ratio(1200u128, 1u128)),
        Some(Decimal::percent(1)),
        Asset {
            info: offer_asset_info.clone(),
            amount: Uint128::from(1200000000u128),
        },
        Asset {
            info: ask_asset_info.clone(),
            amount: Uint128::from(990000u128),
        },
        Uint128::zero(),
        6u8,
        6u8,
    )
    .unwrap();

    assert_max_spread(
        None,
        Some(Decimal::percent(1)),
        Asset {
            info: offer_asset_info.clone(),
            amount: Uint128::zero(),
        },
        Asset {
            info: ask_asset_info.clone(),
            amount: Uint128::from(989999u128),
        },
        Uint128::from(10001u128),
        6u8,
        6u8,
    )
    .unwrap_err();

    assert_max_spread(
        None,
        Some(Decimal::percent(1)),
        Asset {
            info: offer_asset_info,
            amount: Uint128::zero(),
        },
        Asset {
            info: ask_asset_info,
            amount: Uint128::from(990000u128),
        },
        Uint128::from(10000u128),
        6u8,
        6u8,
    )
    .unwrap();
}

#[test]
fn test_max_spread_with_diff_decimal() {
    let token_addr = "ask_asset_info".to_string();

    let mut deps = mock_dependencies(&[]);
    deps.querier.with_token_balances(&[(
        &token_addr,
        &[(
            &MOCK_CONTRACT_ADDR.to_string(),
            &Uint128::from(10000000000u64),
        )],
    )]);
    let offer_asset_info = AssetInfo::NativeToken {
        denom: "offer_asset".to_string(),
    };
    let ask_asset_info = AssetInfo::Token {
        contract_addr: token_addr.to_string(),
    };

    assert_max_spread(
        Some(Decimal::from_ratio(1200u128, 1u128)),
        Some(Decimal::percent(1)),
        Asset {
            info: offer_asset_info.clone(),
            amount: Uint128::from(1200000000u128),
        },
        Asset {
            info: ask_asset_info.clone(),
            amount: Uint128::from(100000000u128),
        },
        Uint128::zero(),
        6u8,
        8u8,
    )
    .unwrap();

    assert_max_spread(
        Some(Decimal::from_ratio(1200u128, 1u128)),
        Some(Decimal::percent(1)),
        Asset {
            info: offer_asset_info,
            amount: Uint128::from(1200000000u128),
        },
        Asset {
            info: ask_asset_info,
            amount: Uint128::from(98999999u128),
        },
        Uint128::zero(),
        6u8,
        8u8,
    )
    .unwrap_err();

    let offer_asset_info = AssetInfo::Token {
        contract_addr: token_addr,
    };
    let ask_asset_info = AssetInfo::NativeToken {
        denom: "offer_asset".to_string(),
    };

    assert_max_spread(
        Some(Decimal::from_ratio(1200u128, 1u128)),
        Some(Decimal::percent(1)),
        Asset {
            info: offer_asset_info.clone(),
            amount: Uint128::from(120000000000u128),
        },
        Asset {
            info: ask_asset_info.clone(),
            amount: Uint128::from(1000000u128),
        },
        Uint128::zero(),
        8u8,
        6u8,
    )
    .unwrap();

    assert_max_spread(
        Some(Decimal::from_ratio(1200u128, 1u128)),
        Some(Decimal::percent(1)),
        Asset {
            info: offer_asset_info,
            amount: Uint128::from(120000000000u128),
        },
        Asset {
            info: ask_asset_info,
            amount: Uint128::from(989999u128),
        },
        Uint128::zero(),
        8u8,
        6u8,
    )
    .unwrap_err();
}

#[test]
fn test_query_pool() {
    let total_share_amount = Uint128::from(111u128);
    let asset_0_amount = Uint128::from(222u128);
    let asset_1_amount = Uint128::from(333u128);
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: asset_0_amount,
    }]);

    deps.querier.with_token_balances(&[
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &asset_1_amount)],
        ),
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &total_share_amount)],
        ),
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
        },
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

    let res: PoolResponse = query_pool(deps.as_ref()).unwrap();

    assert_eq!(
        res.assets,
        [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: asset_0_amount,
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: "asset0000".to_string(),
                },
                amount: asset_1_amount,
            }
        ]
    );
    assert_eq!(res.total_share, total_share_amount);
}

#[test]
fn test_compute_swap_with_huge_pool_variance() {
    let offer_pool = Uint128::from(395451850234u128);
    let ask_pool = Uint128::from(317u128);
    let pool_fees = PoolFee {
        protocol_fee: Fee {
            share: Decimal::percent(1u64),
        },
        swap_fee: Fee {
            share: Decimal::percent(1u64),
        },
    };

    assert_eq!(
        compute_swap(offer_pool, ask_pool, Uint128::from(1u128), pool_fees).return_amount,
        Uint128::zero()
    );
}

#[test]
fn test_update_cofig_unsuccessful() {
    let mut deps = mock_dependencies(&[]);

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
        },
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    // we can just call .unwrap() to assert this was a success
    instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // an unauthorized party tries to update the config
    let info = mock_info("unauthorized", &[]);
    let update_config_message = UpdateConfig {
        owner: Some("unauthorized".to_string()),
        pool_fees: None,
        feature_toggle: None,
    };

    let res = execute(deps.as_mut(), env, info, update_config_message);
    match res {
        Ok(_) => panic!("should return Std(GenericErr -> msg: unauthorized)"),
        Err(ContractError::Std { .. }) => (),
        _ => panic!("should return Std(GenericErr -> msg: unauthorized)"),
    }
}

#[test]
fn test_update_cofig_successful() {
    let mut deps = mock_dependencies(&[]);

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
                share: Decimal::zero(),
            },
            swap_fee: Fee {
                share: Decimal::zero(),
            },
        },
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    // we can just call .unwrap() to assert this was a success
    instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let config = query_config(deps.as_ref()).unwrap();

    // check for original config
    assert_eq!(config.owner, Addr::unchecked("addr0000"));
    assert_eq!(config.feature_toggle.swaps_enabled, true);
    assert_eq!(config.pool_fees.swap_fee.share, Decimal::zero());

    let update_config_message = UpdateConfig {
        owner: Some("new_admin".to_string()),
        pool_fees: Some(PoolFee {
            protocol_fee: Fee {
                share: Decimal::percent(1u64),
            },
            swap_fee: Fee {
                share: Decimal::percent(3u64),
            },
        }),
        feature_toggle: None,
    };

    execute(deps.as_mut(), env, info, update_config_message).unwrap();

    let config = query_config(deps.as_ref()).unwrap();

    // check for new config
    assert_eq!(config.owner, Addr::unchecked("new_admin"));
    assert_eq!(config.pool_fees.swap_fee.share, Decimal::percent(3u64));
}

#[test]
fn test_feature_toggle_swap_disabled() {
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
                share: Decimal::percent(3u64),
            },
        },
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    // we can just call .unwrap() to assert this was a success
    instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // all features are enabled by default, let's disable swaps
    let update_config_message = UpdateConfig {
        owner: None,
        pool_fees: None,
        feature_toggle: Some(FeatureToggle {
            withdrawals_enabled: true,
            deposits_enabled: true,
            swaps_enabled: false,
        }),
    };
    execute(deps.as_mut(), env.clone(), info, update_config_message).unwrap();

    // swap offering NativeToken should fail
    let offer_amount = Uint128::from(1500000000u128);
    let msg = ExecuteMsg::Swap {
        offer_asset: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: offer_amount,
        },
        belief_price: None,
        max_spread: None,
        to: None,
    };
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: offer_amount,
        }],
    );
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);

    match res {
        Ok(_) => panic!("should return ContractError::OperationDisabled(swap)"),
        Err(ContractError::OperationDisabled { .. }) => (),
        _ => panic!("should return ContractError::OperationDisabled(swap)"),
    }

    // swap offering Token should fail
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        amount: offer_amount,
        msg: to_binary(&Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: None,
            to: None,
        })
        .unwrap(),
    });
    let res = execute(deps.as_mut(), env, info, msg);
    match res {
        Ok(_) => panic!("should return ContractError::OperationDisabled(swap)"),
        Err(ContractError::OperationDisabled { .. }) => (),
        _ => panic!("should return ContractError::OperationDisabled(swap)"),
    }
}

#[test]
fn test_feature_toggle_withdrawals_disabled() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(100u128),
    }]);

    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&"addr0000".to_string(), &Uint128::from(100u128))],
        ),
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(100u128))],
        ),
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
        },
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

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

    // all features are enabled by default, let's disable withdrawals
    let update_config_message = UpdateConfig {
        owner: None,
        pool_fees: None,
        feature_toggle: Some(FeatureToggle {
            withdrawals_enabled: false,
            deposits_enabled: true,
            swaps_enabled: true,
        }),
    };
    execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        update_config_message,
    )
    .unwrap();

    // withdraw liquidity should fail
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        msg: to_binary(&Cw20HookMsg::WithdrawLiquidity {}).unwrap(),
        amount: Uint128::from(100u128),
    });

    let env = mock_env();
    let info = mock_info("liquidity0000", &[]);
    let res = execute(deps.as_mut(), env, info, msg);
    match res {
        Ok(_) => panic!("should return OperationDisabled(withdraw_liquidity)"),
        Err(ContractError::OperationDisabled { .. }) => (),
        _ => panic!("should return OperationDisabled(withdraw_liquidity)"),
    }
}

#[test]
fn test_feature_toggle_deposits_disabled() {
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
        },
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

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

    // all features are enabled by default, let's disable deposits
    let update_config_message = UpdateConfig {
        owner: None,
        pool_fees: None,
        feature_toggle: Some(FeatureToggle {
            withdrawals_enabled: true,
            deposits_enabled: false,
            swaps_enabled: true,
        }),
    };
    execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        update_config_message,
    )
    .unwrap();

    // provide liquidity should fail
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
        Ok(_) => panic!("should return OperationDisabled(provide_liquidity)"),
        Err(ContractError::OperationDisabled { .. }) => (),
        _ => panic!("should return OperationDisabled(provide_liquidity)"),
    }
}

#[test]
fn test_protocol_fees() {
    let total_share = Uint128::from(30_000_000_000u128);
    let asset_pool_amount = Uint128::from(20_000_000_000u128);
    let collateral_pool_amount = Uint128::from(30_000_000_000u128);
    let offer_amount = Uint128::from(1_500_000_000u128);

    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: collateral_pool_amount + offer_amount,
        /* user deposit must be pre-applied */
    }]);

    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &total_share)],
        ),
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &asset_pool_amount)],
        ),
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
                share: Decimal::from_ratio(1u128, 1000u128),
            },
            swap_fee: Fee {
                share: Decimal::from_ratio(3u128, 1000u128),
            },
        },
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

    // first swap
    let msg = ExecuteMsg::Swap {
        offer_asset: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: offer_amount,
        },
        belief_price: None,
        max_spread: None,
        to: None,
    };
    let env = mock_env();
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: offer_amount,
        }],
    );
    execute(deps.as_mut(), env, info, msg).unwrap();

    // ask_amount = (ask_pool * offer_amount / (offer_pool + offer_amount))
    // 952.380952 = 20000 * 1500 / (30000 + 1500) - swap_fee - protocol_fee
    let expected_ret_amount = Uint128::from(952_380_952u128);
    let expected_protocol_fee_amount = expected_ret_amount.multiply_ratio(1u128, 1000u128); // 0.1%

    // as we swapped native to token, we accumulate the protocol fees in token. Native token fees should be 0
    let protocol_fees_for_token =
        query_protocol_fees(deps.as_ref(), Some("asset0000".to_string()), None)
            .unwrap()
            .fees;
    assert_eq!(
        protocol_fees_for_token.first().unwrap().amount,
        expected_protocol_fee_amount
    );
    let protocol_fees_for_native =
        query_protocol_fees(deps.as_ref(), Some("uusd".to_string()), None)
            .unwrap()
            .fees;
    assert_eq!(
        protocol_fees_for_native.first().unwrap().amount,
        Uint128::zero()
    );

    // second swap
    let msg = ExecuteMsg::Swap {
        offer_asset: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: offer_amount,
        },
        belief_price: None,
        max_spread: None,
        to: None,
    };
    let env = mock_env();
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: offer_amount,
        }],
    );
    execute(deps.as_mut(), env, info, msg).unwrap();

    let expected_ret_amount = Uint128::from(1_904_760_000u128);
    let new_expected_protocol_fee_amount = expected_ret_amount.multiply_ratio(1u128, 1000u128); // 0.1%

    // the new protocol fees should have increased from the previous time
    let new_protocol_fees_for_token =
        query_protocol_fees(deps.as_ref(), Some("asset0000".to_string()), None)
            .unwrap()
            .fees;
    assert!(new_protocol_fees_for_token.first().unwrap().amount > expected_protocol_fee_amount);
    assert_eq!(
        new_protocol_fees_for_token.first().unwrap().amount,
        new_expected_protocol_fee_amount
    );
    let protocol_fees_for_native =
        query_protocol_fees(deps.as_ref(), Some("uusd".to_string()), None)
            .unwrap()
            .fees;
    assert_eq!(
        protocol_fees_for_native.first().unwrap().amount,
        Uint128::zero()
    );
}

#[test]
fn test_collect_protocol_fees_successful() {
    let total_share = Uint128::from(30_000_000_000u128);
    let asset_pool_amount = Uint128::from(20_000_000_000u128);
    let collateral_pool_amount = Uint128::from(30_000_000_000u128);
    let offer_amount = Uint128::from(1_500_000_000u128);

    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: collateral_pool_amount + offer_amount,
        /* user deposit must be pre-applied */
    }]);

    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &total_share)],
        ),
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &asset_pool_amount)],
        ),
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
                share: Decimal::from_ratio(1u128, 1000u128),
            },
            swap_fee: Fee {
                share: Decimal::from_ratio(3u128, 1000u128),
            },
        },
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

    // first swap, native -> token
    let msg = ExecuteMsg::Swap {
        offer_asset: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: offer_amount,
        },
        belief_price: None,
        max_spread: None,
        to: None,
    };
    let env = mock_env();
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: offer_amount,
        }],
    );
    execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // ask_amount = (ask_pool * offer_amount / (offer_pool + offer_amount))
    // 952.380952 = 20000 * 1500 / (30000 + 1500) - swap_fee - protocol_fee
    let expected_ret_amount = Uint128::from(952_380_952u128);
    let expected_protocol_fee_token_amount = expected_ret_amount.multiply_ratio(1u128, 1000u128); // 0.1%

    // second swap, token -> native
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        amount: offer_amount,
        msg: to_binary(&Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: None,
            to: None,
        })
        .unwrap(),
    });
    let info = mock_info("asset0000", &[]);
    execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let expected_ret_amount = Uint128::from(787_500_000u128);
    let expected_protocol_fee_native_amount = expected_ret_amount.multiply_ratio(3u128, 1000u128); // 0.1%

    // as we swapped both native and token, we should have collected fees in both of them
    let protocol_fees_for_token =
        query_protocol_fees(deps.as_ref(), Some("asset0000".to_string()), None)
            .unwrap()
            .fees;
    assert_eq!(
        protocol_fees_for_token.first().unwrap().amount,
        expected_protocol_fee_token_amount
    );
    let protocol_fees_for_native =
        query_protocol_fees(deps.as_ref(), Some("uusd".to_string()), None)
            .unwrap()
            .fees;
    assert_eq!(
        protocol_fees_for_native.first().unwrap().amount,
        expected_protocol_fee_native_amount
    );

    // collect the fees
    let info = mock_info("addr0000", &[]);
    let msg = ExecuteMsg::CollectProtocolFees {};
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    // make sure two messages were sent, one for the native token and one for the cw20
    assert_eq!(res.messages.len(), 2);

    let transfer_native_token_msg = res.messages.get(0).expect("no message");
    let transfer_cw20_token_msg = res.messages.get(1).expect("no message");
    assert_eq!(
        transfer_native_token_msg,
        &SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "addr0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: protocol_fees_for_native.clone().first().unwrap().amount,
            }],
        }))
    );
    assert_eq!(
        transfer_cw20_token_msg,
        &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset0000".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "addr0000".to_string(),
                amount: protocol_fees_for_token.clone().first().unwrap().amount,
            })
            .unwrap(),
            funds: vec![],
        }))
    );

    // now collected protocol fees should be reset to zero
    let protocol_fees_for_token =
        query_protocol_fees(deps.as_ref(), Some("asset0000".to_string()), None)
            .unwrap()
            .fees;
    assert_eq!(
        protocol_fees_for_token.first().unwrap().amount,
        Uint128::zero()
    );
    let protocol_fees_for_native =
        query_protocol_fees(deps.as_ref(), Some("uusd".to_string()), None)
            .unwrap()
            .fees;
    assert_eq!(
        protocol_fees_for_native.first().unwrap().amount,
        Uint128::zero()
    );

    // all time collected protocol fees should remain intact
    let all_time_protocol_fees = query_protocol_fees(deps.as_ref(), None, Some(true))
        .unwrap()
        .fees;

    assert_eq!(
        all_time_protocol_fees[0].amount,
        expected_protocol_fee_native_amount
    );
    assert_eq!(
        all_time_protocol_fees[1].amount,
        expected_protocol_fee_token_amount
    );
}

#[test]
fn test_collect_protocol_fees_unsuccessful() {
    let total_share = Uint128::from(30_000_000_000u128);
    let asset_pool_amount = Uint128::from(20_000_000_000u128);
    let collateral_pool_amount = Uint128::from(30_000_000_000u128);
    let offer_amount = Uint128::from(1_500_000_000u128);

    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: collateral_pool_amount + offer_amount,
        /* user deposit must be pre-applied */
    }]);

    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &total_share)],
        ),
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &asset_pool_amount)],
        ),
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
                share: Decimal::from_ratio(1u128, 1000u128),
            },
            swap_fee: Fee {
                share: Decimal::from_ratio(3u128, 1000u128),
            },
        },
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

    // first swap, native -> token
    let msg = ExecuteMsg::Swap {
        offer_asset: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: offer_amount,
        },
        belief_price: None,
        max_spread: None,
        to: None,
    };
    let env = mock_env();
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: offer_amount,
        }],
    );
    execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // ask_amount = (ask_pool * offer_amount / (offer_pool + offer_amount))
    // 952.380952 = 20000 * 1500 / (30000 + 1500) - swap_fee - protocol_fee
    let expected_ret_amount = Uint128::from(952_380_952u128);
    let expected_protocol_fee_token_amount = expected_ret_amount.multiply_ratio(1u128, 1000u128); // 0.1%

    // second swap, token -> native
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        amount: offer_amount,
        msg: to_binary(&Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: None,
            to: None,
        })
        .unwrap(),
    });
    let info = mock_info("asset0000", &[]);
    execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let expected_ret_amount = Uint128::from(787_500_000u128);
    let expected_protocol_fee_native_amount = expected_ret_amount.multiply_ratio(3u128, 1000u128); // 0.1%

    // as we swapped both native and token, we should have collected fees in both of them
    let protocol_fees_for_token =
        query_protocol_fees(deps.as_ref(), Some("asset0000".to_string()), None)
            .unwrap()
            .fees;
    assert_eq!(
        protocol_fees_for_token.first().unwrap().amount,
        expected_protocol_fee_token_amount
    );
    let protocol_fees_for_native =
        query_protocol_fees(deps.as_ref(), Some("uusd".to_string()), None)
            .unwrap()
            .fees;
    assert_eq!(
        protocol_fees_for_native.first().unwrap().amount,
        expected_protocol_fee_native_amount
    );

    // try collecting the fees by an unauthorized address
    let info = mock_info("unauthorized", &[]);
    let msg = ExecuteMsg::CollectProtocolFees {};
    let res = execute(deps.as_mut(), env, info, msg);

    match res {
        Ok(_) => panic!("should return Std(GenericErr -> msg: unauthorized)"),
        Err(ContractError::Std { .. }) => (),
        _ => panic!("should return Std(GenericErr -> msg: unauthorized)"),
    }
}
