use crate::contract::{execute, instantiate, reply};
use crate::queries::query_fees;
use crate::state::{ALL_TIME_COLLECTED_PROTOCOL_FEES, COLLECTED_PROTOCOL_FEES};
use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    to_binary, BankMsg, Coin, CosmosMsg, Decimal, Reply, StdError, SubMsg, SubMsgResponse,
    SubMsgResult, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use pool_network::asset::{Asset, AssetInfo, PairType};
use pool_network::mock_querier::mock_dependencies;
use pool_network::pair::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, PoolFee};
use white_whale::fee::Fee;

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

    // ask_amount = ((ask_pool - accrued protocol fees) * offer_amount / (offer_pool - accrued protocol fees + offer_amount))
    // 952.380952 = (20000 - 0) * 1500 / (30000 - 0 + 1500) - swap_fee - protocol_fee
    let expected_ret_amount = Uint128::from(952_380_952u128);
    let expected_protocol_fee_amount = expected_ret_amount.multiply_ratio(1u128, 1000u128); // 0.1%

    // as we swapped native to token, we accumulate the protocol fees in token. Native token fees should be 0
    let protocol_fees_for_token = query_fees(
        deps.as_ref(),
        Some("asset0000".to_string()),
        None,
        COLLECTED_PROTOCOL_FEES,
        Some(ALL_TIME_COLLECTED_PROTOCOL_FEES),
    )
    .unwrap()
    .fees;
    assert_eq!(
        protocol_fees_for_token.first().unwrap().amount,
        expected_protocol_fee_amount
    );
    let protocol_fees_for_native = query_fees(
        deps.as_ref(),
        Some("uusd".to_string()),
        None,
        COLLECTED_PROTOCOL_FEES,
        Some(ALL_TIME_COLLECTED_PROTOCOL_FEES),
    )
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

    // ask_amount = ((ask_pool - accrued protocol fees) * offer_amount / (offer_pool - accrued protocol fees + offer_amount))
    // 952.335600 = (20000 - 0.952380 ) * 1500 / (30000 - 0 + 1500) - swap_fee - protocol_fee
    let expected_ret_amount = Uint128::from(952_335_600u128);
    let new_expected_protocol_fee_amount = expected_ret_amount.multiply_ratio(1u128, 1000u128); // 0.1%

    // the new protocol fees should have increased from the previous time
    let new_protocol_fees_for_token = query_fees(
        deps.as_ref(),
        Some("asset0000".to_string()),
        None,
        COLLECTED_PROTOCOL_FEES,
        Some(ALL_TIME_COLLECTED_PROTOCOL_FEES),
    )
    .unwrap()
    .fees;
    assert!(new_protocol_fees_for_token.first().unwrap().amount > expected_protocol_fee_amount);
    assert_eq!(
        new_protocol_fees_for_token.first().unwrap().amount,
        new_expected_protocol_fee_amount + expected_protocol_fee_amount // fees collected in the first + second swap
    );
    let protocol_fees_for_native = query_fees(
        deps.as_ref(),
        Some("uusd".to_string()),
        None,
        COLLECTED_PROTOCOL_FEES,
        Some(ALL_TIME_COLLECTED_PROTOCOL_FEES),
    )
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

    // ask_amount = ((ask_pool - accrued protocol fees) * offer_amount / (offer_pool - accrued protocol fees + offer_amount))
    // 952.380952 = (20000 - 0) * 1500 / (30000 - 0 + 1500) - swap_fee - protocol_fee
    let expected_ret_amount = Uint128::from(952_380_952u128);
    let expected_protocol_fee_token_amount = expected_ret_amount.multiply_ratio(1u128, 1000u128); // 0.001%

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

    // ask_amount = ((ask_pool - accrued protocol fees) * offer_amount / (offer_pool - accrued protocol fees + offer_amount))
    // 2362.612505 = (31500 - 0) * 1500 / (18500 - 0.952380 + 1500) - swap_fee - protocol_fee
    let expected_ret_amount = Uint128::from(2_362_612_505u128);
    let expected_protocol_fee_native_amount = expected_ret_amount.multiply_ratio(1u128, 1000u128); // 0.001%

    // as we swapped both native and token, we should have collected fees in both of them
    let protocol_fees_for_token = query_fees(
        deps.as_ref(),
        Some("asset0000".to_string()),
        None,
        COLLECTED_PROTOCOL_FEES,
        Some(ALL_TIME_COLLECTED_PROTOCOL_FEES),
    )
    .unwrap()
    .fees;

    assert_eq!(
        protocol_fees_for_token.first().unwrap().amount,
        expected_protocol_fee_token_amount
    );
    let protocol_fees_for_native = query_fees(
        deps.as_ref(),
        Some("uusd".to_string()),
        None,
        COLLECTED_PROTOCOL_FEES,
        Some(ALL_TIME_COLLECTED_PROTOCOL_FEES),
    )
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
            to_address: "collector".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: protocol_fees_for_native.first().unwrap().amount,
            }],
        }))
    );
    assert_eq!(
        transfer_cw20_token_msg,
        &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset0000".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "collector".to_string(),
                amount: protocol_fees_for_token.first().unwrap().amount,
            })
            .unwrap(),
            funds: vec![],
        }))
    );

    // now collected protocol fees should be reset to zero
    let protocol_fees_for_token = query_fees(
        deps.as_ref(),
        Some("asset0000".to_string()),
        None,
        COLLECTED_PROTOCOL_FEES,
        Some(ALL_TIME_COLLECTED_PROTOCOL_FEES),
    )
    .unwrap()
    .fees;
    assert_eq!(
        protocol_fees_for_token.first().unwrap().amount,
        Uint128::zero()
    );
    let protocol_fees_for_native = query_fees(
        deps.as_ref(),
        Some("uusd".to_string()),
        None,
        COLLECTED_PROTOCOL_FEES,
        Some(ALL_TIME_COLLECTED_PROTOCOL_FEES),
    )
    .unwrap()
    .fees;
    assert_eq!(
        protocol_fees_for_native.first().unwrap().amount,
        Uint128::zero()
    );

    // all time collected protocol fees should remain intact
    let all_time_protocol_fees = query_fees(
        deps.as_ref(),
        None,
        Some(true),
        COLLECTED_PROTOCOL_FEES,
        Some(ALL_TIME_COLLECTED_PROTOCOL_FEES),
    )
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
fn test_collect_protocol_fees_successful_1_fee_only() {
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

    // swap native -> token
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

    // as did only one swap from native -> token, we should have collected fees in token
    let protocol_fees = query_fees(
        deps.as_ref(),
        None,
        None,
        COLLECTED_PROTOCOL_FEES,
        Some(ALL_TIME_COLLECTED_PROTOCOL_FEES),
    )
    .unwrap()
    .fees;
    assert_eq!(protocol_fees[0].amount, Uint128::zero());
    assert_eq!(protocol_fees[1].amount, expected_protocol_fee_token_amount);

    // collect the fees
    let info = mock_info("addr0000", &[]);
    let msg = ExecuteMsg::CollectProtocolFees {};
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    // make sure one message was sent, as there is only one fee to collect, the other one is zero
    assert_eq!(res.messages.len(), 1);

    let transfer_cw20_token_msg = res.messages.get(0).expect("no message");
    assert_eq!(
        transfer_cw20_token_msg,
        &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset0000".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "collector".to_string(),
                amount: protocol_fees[1].amount,
            })
            .unwrap(),
            funds: vec![],
        }))
    );

    // now collected protocol fees should be reset to zero
    let protocol_fees_for_token = query_fees(
        deps.as_ref(),
        Some("asset0000".to_string()),
        None,
        COLLECTED_PROTOCOL_FEES,
        Some(ALL_TIME_COLLECTED_PROTOCOL_FEES),
    )
    .unwrap()
    .fees;
    assert_eq!(
        protocol_fees_for_token.first().unwrap().amount,
        Uint128::zero()
    );
    let protocol_fees_for_native = query_fees(
        deps.as_ref(),
        Some("uusd".to_string()),
        None,
        COLLECTED_PROTOCOL_FEES,
        Some(ALL_TIME_COLLECTED_PROTOCOL_FEES),
    )
    .unwrap()
    .fees;
    assert_eq!(
        protocol_fees_for_native.first().unwrap().amount,
        Uint128::zero()
    );

    // all time collected protocol fees should remain intact
    let all_time_protocol_fees = query_fees(
        deps.as_ref(),
        None,
        Some(true),
        COLLECTED_PROTOCOL_FEES,
        Some(ALL_TIME_COLLECTED_PROTOCOL_FEES),
    )
    .unwrap()
    .fees;

    assert_eq!(all_time_protocol_fees[0].amount, Uint128::zero());
    assert_eq!(
        all_time_protocol_fees[1].amount,
        expected_protocol_fee_token_amount
    );
}

#[test]
fn protocol_fees() {
    let protocol_fee = PoolFee {
        protocol_fee: Fee {
            share: Decimal::percent(50),
        },
        swap_fee: Fee {
            share: Decimal::percent(50),
        },
        burn_fee: Fee {
            share: Decimal::zero(),
        },
    };
    assert_eq!(
        protocol_fee.is_valid(),
        Err(StdError::generic_err("Invalid fees"))
    );

    let protocol_fee = PoolFee {
        protocol_fee: Fee {
            share: Decimal::percent(200),
        },
        swap_fee: Fee {
            share: Decimal::percent(20),
        },
        burn_fee: Fee {
            share: Decimal::zero(),
        },
    };
    assert_eq!(
        protocol_fee.is_valid(),
        Err(StdError::generic_err("Invalid fee"))
    );

    let protocol_fee = PoolFee {
        protocol_fee: Fee {
            share: Decimal::percent(20),
        },
        swap_fee: Fee {
            share: Decimal::percent(200),
        },
        burn_fee: Fee {
            share: Decimal::zero(),
        },
    };
    assert_eq!(
        protocol_fee.is_valid(),
        Err(StdError::generic_err("Invalid fee"))
    );

    let protocol_fee = PoolFee {
        protocol_fee: Fee {
            share: Decimal::percent(40),
        },
        swap_fee: Fee {
            share: Decimal::percent(60),
        },
        burn_fee: Fee {
            share: Decimal::zero(),
        },
    };
    assert_eq!(
        protocol_fee.is_valid(),
        Err(StdError::generic_err("Invalid fees"))
    );

    let protocol_fee = PoolFee {
        protocol_fee: Fee {
            share: Decimal::percent(20),
        },
        swap_fee: Fee {
            share: Decimal::percent(60),
        },
        burn_fee: Fee {
            share: Decimal::zero(),
        },
    };
    assert_eq!(protocol_fee.is_valid(), Ok(()));
}
