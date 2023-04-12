use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    attr, coin, to_binary, BankMsg, Coin, CosmosMsg, Decimal, Reply, Response, SubMsg,
    SubMsgResponse, SubMsgResult, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use white_whale::fee::Fee;
use white_whale::pool_network;
use white_whale::pool_network::asset::{AssetInfo, PairType};
use white_whale::pool_network::denom::MsgBurn;
use white_whale::pool_network::mock_querier::mock_dependencies;
use white_whale::pool_network::pair::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, PoolFee};

use crate::contract::{execute, instantiate, reply};
use crate::error::ContractError;
use crate::state::LP_SYMBOL;
use crate::state::{get_fees_for_asset, store_fee, COLLECTED_PROTOCOL_FEES};

#[test]
fn withdraw_xyk_liquidity_cw20_lp() {
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
            burn_fee: Fee {
                share: Decimal::zero(),
            },
        },
        fee_collector_addr: "collector".to_string(),
        pair_type: PairType::ConstantProduct,
        token_factory_lp: false,
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
    store_fee(
        deps.as_mut().storage,
        Uint128::from(10u8),
        "uusd".to_string(),
        COLLECTED_PROTOCOL_FEES,
    )
    .unwrap();
    store_fee(
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
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    let log_withdrawn_share = res.attributes.get(2).expect("no log");
    let log_refund_assets = res.attributes.get(3).expect("no log");
    let msg_refund_0 = res.messages.get(0).expect("no message");
    let msg_refund_1 = res.messages.get(1).expect("no message");
    let msg_burn_liquidity = res.messages.get(2).expect("no message");

    let protocol_fee_native = get_fees_for_asset(
        deps.as_mut().storage,
        "uusd".to_string(),
        COLLECTED_PROTOCOL_FEES,
    )
    .unwrap();
    let expected_native_refund_amount: Uint128 = Uint128::from(100u128)
        .checked_sub(protocol_fee_native.amount)
        .unwrap();

    let protocol_fee_token = get_fees_for_asset(
        deps.as_mut().storage,
        "asset0000".to_string(),
        COLLECTED_PROTOCOL_FEES,
    )
    .unwrap();
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
fn withdraw_stableswap_liquidity() {
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
            burn_fee: Fee {
                share: Decimal::zero(),
            },
        },
        fee_collector_addr: "collector".to_string(),
        pair_type: PairType::StableSwap { amp: 100 },
        token_factory_lp: false,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    instantiate(deps.as_mut(), env, info, msg).unwrap();

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
    reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    // store some protocol fees in both native and token
    store_fee(
        deps.as_mut().storage,
        Uint128::from(10u8),
        "uusd".to_string(),
        COLLECTED_PROTOCOL_FEES,
    )
    .unwrap();
    store_fee(
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
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    let protocol_fee_native = get_fees_for_asset(
        deps.as_mut().storage,
        "uusd".to_string(),
        COLLECTED_PROTOCOL_FEES,
    )
    .unwrap();
    let expected_native_refund_amount: Uint128 = Uint128::from(100u128)
        .checked_sub(protocol_fee_native.amount)
        .unwrap();

    let protocol_fee_token = get_fees_for_asset(
        deps.as_mut().storage,
        "asset0000".to_string(),
        COLLECTED_PROTOCOL_FEES,
    )
    .unwrap();
    let expected_token_refund_amount: Uint128 = Uint128::from(100u128)
        .checked_sub(protocol_fee_token.amount)
        .unwrap();

    assert_eq!(
        res,
        Response::new()
            .add_messages(vec![
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: "addr0000".to_string(),
                    amount: vec![Coin {
                        denom: "uusd".to_string(),
                        amount: expected_native_refund_amount,
                    }],
                }),
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "asset0000".to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: "addr0000".to_string(),
                        amount: expected_token_refund_amount,
                    })
                    .unwrap(),
                    funds: vec![],
                }),
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "liquidity0000".to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Burn {
                        amount: Uint128::from(100u128),
                    })
                    .unwrap(),
                    funds: vec![],
                }),
            ])
            .add_attributes(vec![
                ("action", "withdraw_liquidity"),
                ("sender", "addr0000"),
                ("withdrawn_share", "100"),
                ("refund_assets", ("90uusd, 80asset0000")),
            ])
    );
}

#[test]
fn test_withdrawal_unauthorized() {
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
            burn_fee: Fee {
                share: Decimal::zero(),
            },
        },
        fee_collector_addr: "collector".to_string(),
        pair_type: PairType::ConstantProduct,
        token_factory_lp: false,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    instantiate(deps.as_mut(), env, info, msg).unwrap();

    // withdraw liquidity should fail
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        msg: to_binary(&Cw20HookMsg::WithdrawLiquidity {}).unwrap(),
        amount: Uint128::from(100u128),
    });

    let env = mock_env();
    let info = mock_info("not_cw20", &[]);
    let res = execute(deps.as_mut(), env, info, msg);
    match res {
        Ok(_) => panic!("should return ContractError::Unauthorized"),
        Err(ContractError::Unauthorized { .. }) => (),
        _ => panic!("should return ContractError::Unauthorized"),
    }
}

#[test]
fn test_withdrawal_wrong_message() {
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
            burn_fee: Fee {
                share: Decimal::zero(),
            },
        },
        fee_collector_addr: "collector".to_string(),
        pair_type: PairType::ConstantProduct,
        token_factory_lp: false,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    instantiate(deps.as_mut(), env, info, msg).unwrap();

    // withdraw liquidity should fail
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        msg: to_binary(&"invalid_message").unwrap(),
        amount: Uint128::from(100u128),
    });

    let env = mock_env();
    let info = mock_info("liquidity0000", &[]);
    let res = execute(deps.as_mut(), env, info, msg);
    match res {
        Ok(_) => panic!("should return ContractError::Std"),
        Err(ContractError::Std { .. }) => (),
        _ => panic!("should return ContractError::Std"),
    }
}

#[test]
fn withdraw_xyk_liquidity_token_factory_lp() {
    let lp_denom = format!("{}/{MOCK_CONTRACT_ADDR}/{LP_SYMBOL}", "factory");

    let mut deps = mock_dependencies(&[
        Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(2000u128),
        },
        Coin {
            denom: "uwhale".to_string(),
            amount: Uint128::from(2000u128),
        },
        Coin {
            denom: lp_denom.clone(),
            amount: Uint128::from(1000u128),
        },
    ]);

    deps.querier.with_balance(&[(
        &"addr0000".to_string(),
        vec![Coin {
            denom: lp_denom.clone(),
            amount: Uint128::from(1000u128 /* user deposit must be pre-applied */),
        }],
    )]);

    let msg = InstantiateMsg {
        asset_infos: [
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
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
        token_factory_lp: true,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    // withdraw liquidity
    let msg = ExecuteMsg::WithdrawLiquidity {};

    let env = mock_env();
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: lp_denom.clone(),
            amount: Uint128::from(1000u128),
        }],
    );
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    let log_withdrawn_share = res.attributes.get(2).expect("no log");
    let log_refund_assets = res.attributes.get(3).expect("no log");
    let msg_refund_0 = res.messages.get(0).expect("no message").clone().msg;
    let msg_refund_1 = res.messages.get(1).expect("no message").clone().msg;
    let msg_burn_liquidity = res.messages.get(2).expect("no message").clone().msg;

    let expected_asset_0_refund_amount: Uint128 = Uint128::from(1000u128);
    let expected_asset_1_refund_amount: Uint128 = Uint128::from(1000u128);

    let msg_refund_0_expected = CosmosMsg::Bank(BankMsg::Send {
        to_address: "addr0000".to_string(),
        amount: vec![coin(expected_asset_0_refund_amount.u128(), "uusd")],
    });
    let msg_refund_1_expected = CosmosMsg::Bank(BankMsg::Send {
        to_address: "addr0000".to_string(),
        amount: vec![coin(expected_asset_1_refund_amount.u128(), "uwhale")],
    });
    let msg_burn_liquidity_expected = <MsgBurn as Into<CosmosMsg>>::into(MsgBurn {
        sender: MOCK_CONTRACT_ADDR.to_string(),
        amount: Some(pool_network::denom::Coin {
            denom: lp_denom.clone(),
            amount: "1000".to_string(),
        }),
    });

    assert_eq!(msg_refund_0, msg_refund_0_expected);
    assert_eq!(msg_refund_1, msg_refund_1_expected);
    assert_eq!(msg_burn_liquidity, msg_burn_liquidity_expected);

    assert_eq!(
        log_withdrawn_share,
        &attr("withdrawn_share", 1000u128.to_string())
    );
    assert_eq!(
        log_refund_assets,
        &attr("refund_assets", "1000uusd, 1000uwhale")
    );
}
#[test]
fn withdraw_xyk_liquidity_token_factory_lp_wrong_asset() {
    let lp_denom = format!("{}/{MOCK_CONTRACT_ADDR}/{LP_SYMBOL}", "factory");

    let mut deps = mock_dependencies(&[
        Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(2000u128),
        },
        Coin {
            denom: "uwhale".to_string(),
            amount: Uint128::from(2000u128),
        },
        Coin {
            denom: lp_denom.clone(),
            amount: Uint128::from(1000u128),
        },
    ]);

    deps.querier.with_balance(&[(
        &"addr0000".to_string(),
        vec![Coin {
            denom: lp_denom.clone(),
            amount: Uint128::from(1000u128 /* user deposit must be pre-applied */),
        }],
    )]);

    let msg = InstantiateMsg {
        asset_infos: [
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
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
        token_factory_lp: true,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    // withdraw liquidity
    let msg = ExecuteMsg::WithdrawLiquidity {};

    let env = mock_env();
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "not_lp_denom".to_string(),
            amount: Uint128::from(1000u128),
        }],
    );
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();

    assert_eq!(err, ContractError::AssetMismatch {});
}
