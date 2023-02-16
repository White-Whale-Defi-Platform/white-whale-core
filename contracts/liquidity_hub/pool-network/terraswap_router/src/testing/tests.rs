use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    attr, coin, from_binary, to_binary, Addr, Coin, CosmosMsg, StdError, SubMsg, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use pool_network::asset::{Asset, AssetInfo, PairInfo, PairType};
use pool_network::mock_querier::mock_dependencies;
use pool_network::pair::ExecuteMsg as PairExecuteMsg;
use pool_network::router::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
    SimulateSwapOperationsResponse, SwapOperation, SwapRoute,
};

use crate::contract::{execute, instantiate, migrate, query};
use crate::error::ContractError;
use crate::operations::asset_into_swap_msg;
use crate::state::SWAP_ROUTES;

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        terraswap_factory: "terraswapfactory".to_string(),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // it worked, let's query the state
    let config: ConfigResponse =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!("terraswapfactory", config.terraswap_factory.as_str());
}

#[test]
fn execute_swap_operations() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_token_balances(&[(
        &"asset0002".to_string(),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(1000000u128))],
    )]);

    let msg = InstantiateMsg {
        terraswap_factory: "terraswapfactory".to_string(),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::ExecuteSwapOperations {
        operations: vec![],
        minimum_receive: None,
        to: None,
    };

    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(ContractError::Std(error)) => assert_eq!(
            error,
            StdError::generic_err("Must provide swap operations to execute")
        ),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let msg = ExecuteMsg::ExecuteSwapOperations {
        operations: vec![
            SwapOperation::TerraSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: "ukrw".to_string(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: "asset0001".to_string(),
                },
            },
            SwapOperation::TerraSwap {
                offer_asset_info: AssetInfo::Token {
                    contract_addr: "asset0001".to_string(),
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
            },
            SwapOperation::TerraSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: "asset0002".to_string(),
                },
            },
        ],
        minimum_receive: Some(Uint128::from(1000000u128)),
        to: None,
    };

    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::ExecuteSwapOperation {
                    operation: SwapOperation::TerraSwap {
                        offer_asset_info: AssetInfo::NativeToken {
                            denom: "ukrw".to_string(),
                        },
                        ask_asset_info: AssetInfo::Token {
                            contract_addr: "asset0001".to_string(),
                        },
                    },
                    to: None,
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::ExecuteSwapOperation {
                    operation: SwapOperation::TerraSwap {
                        offer_asset_info: AssetInfo::Token {
                            contract_addr: "asset0001".to_string(),
                        },
                        ask_asset_info: AssetInfo::NativeToken {
                            denom: "uluna".to_string(),
                        },
                    },
                    to: None,
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::ExecuteSwapOperation {
                    operation: SwapOperation::TerraSwap {
                        offer_asset_info: AssetInfo::NativeToken {
                            denom: "uluna".to_string(),
                        },
                        ask_asset_info: AssetInfo::Token {
                            contract_addr: "asset0002".to_string(),
                        },
                    },
                    to: Some("addr0000".to_string()),
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::AssertMinimumReceive {
                    asset_info: AssetInfo::Token {
                        contract_addr: "asset0002".to_string(),
                    },
                    prev_balance: Uint128::zero(),
                    minimum_receive: Uint128::from(1000000u128),
                    receiver: "addr0000".to_string(),
                })
                .unwrap(),
            })),
        ]
    );

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        amount: Uint128::from(1000000u128),
        msg: to_binary(&Cw20HookMsg::ExecuteSwapOperations {
            operations: vec![
                SwapOperation::TerraSwap {
                    offer_asset_info: AssetInfo::NativeToken {
                        denom: "ukrw".to_string(),
                    },
                    ask_asset_info: AssetInfo::Token {
                        contract_addr: "asset0001".to_string(),
                    },
                },
                SwapOperation::TerraSwap {
                    offer_asset_info: AssetInfo::Token {
                        contract_addr: "asset0001".to_string(),
                    },
                    ask_asset_info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                },
                SwapOperation::TerraSwap {
                    offer_asset_info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    ask_asset_info: AssetInfo::Token {
                        contract_addr: "asset0002".to_string(),
                    },
                },
            ],
            minimum_receive: None,
            to: Some("addr0002".to_string()),
        })
        .unwrap(),
    });

    let info = mock_info("asset0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::ExecuteSwapOperation {
                    operation: SwapOperation::TerraSwap {
                        offer_asset_info: AssetInfo::NativeToken {
                            denom: "ukrw".to_string(),
                        },
                        ask_asset_info: AssetInfo::Token {
                            contract_addr: "asset0001".to_string(),
                        },
                    },
                    to: None,
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::ExecuteSwapOperation {
                    operation: SwapOperation::TerraSwap {
                        offer_asset_info: AssetInfo::Token {
                            contract_addr: "asset0001".to_string(),
                        },
                        ask_asset_info: AssetInfo::NativeToken {
                            denom: "uluna".to_string(),
                        },
                    },
                    to: None,
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::ExecuteSwapOperation {
                    operation: SwapOperation::TerraSwap {
                        offer_asset_info: AssetInfo::NativeToken {
                            denom: "uluna".to_string(),
                        },
                        ask_asset_info: AssetInfo::Token {
                            contract_addr: "asset0002".to_string(),
                        },
                    },
                    to: Some("addr0002".to_string()),
                })
                .unwrap(),
            })),
        ]
    );
}

#[test]
fn execute_swap_operation() {
    let mut deps = mock_dependencies(&[]);
    let msg = InstantiateMsg {
        terraswap_factory: "terraswapfactory".to_string(),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    deps.querier.with_pool_factory(
        &[(
            &"uusdasset0000".to_string(),
            &PairInfo {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    AssetInfo::Token {
                        contract_addr: "asset0000".to_string(),
                    },
                ],
                contract_addr: "pair0000".to_string(),
                liquidity_token: "liquidity0000".to_string(),
                asset_decimals: [6u8, 6u8],
                pair_type: PairType::ConstantProduct,
            },
        )],
        &[("uusd".to_string(), 6u8)],
    );
    deps.querier.with_balance(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        [Coin {
            amount: Uint128::from(1000000u128),
            denom: "uusd".to_string(),
        }]
        .to_vec(),
    )]);

    let msg = ExecuteMsg::ExecuteSwapOperation {
        operation: SwapOperation::TerraSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            ask_asset_info: AssetInfo::Token {
                contract_addr: "asset0000".to_string(),
            },
        },
        to: None,
    };
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    match res {
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(
            asset_into_swap_msg(
                deps.as_ref(),
                Addr::unchecked("pair0000"),
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    amount: Uint128::from(1000000u128),
                },
                None,
                None,
            )
            .unwrap()
        )],
    );

    // optional to address
    // swap_send
    let msg = ExecuteMsg::ExecuteSwapOperation {
        operation: SwapOperation::TerraSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            ask_asset_info: AssetInfo::Token {
                contract_addr: "asset0000".to_string(),
            },
        },
        to: Some("addr0000".to_string()),
    };
    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(
            asset_into_swap_msg(
                deps.as_ref(),
                Addr::unchecked("pair0000"),
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    amount: Uint128::from(1000000u128),
                },
                None,
                Some("addr0000".to_string()),
            )
            .unwrap()
        )],
    );
    deps.querier.with_pool_factory(
        &[(
            &"assetuusd".to_string(),
            &PairInfo {
                asset_infos: [
                    AssetInfo::Token {
                        contract_addr: "asset".to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                ],
                contract_addr: "pair0000".to_string(),
                liquidity_token: "liquidity0000".to_string(),
                asset_decimals: [6u8, 6u8],
                pair_type: PairType::ConstantProduct,
            },
        )],
        &[("uusd".to_string(), 6u8)],
    );
    deps.querier.with_token_balances(&[(
        &"asset".to_string(),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(1000000u128))],
    )]);

    let msg = ExecuteMsg::ExecuteSwapOperation {
        operation: SwapOperation::TerraSwap {
            offer_asset_info: AssetInfo::Token {
                contract_addr: "asset".to_string(),
            },
            ask_asset_info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
        },
        to: Some("addr0000".to_string()),
    };

    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset".to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: "pair0000".to_string(),
                amount: Uint128::from(1000000u128),
                msg: to_binary(&pool_network::pair::Cw20HookMsg::Swap {
                    belief_price: None,
                    max_spread: None,
                    to: Some("addr0000".to_string()),
                })
                .unwrap(),
            })
            .unwrap(),
        }))]
    );
}

#[test]
fn query_buy_with_routes() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        terraswap_factory: "terraswapfactory".to_string(),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // try simulating with empty operations
    let empty_operations_msg = QueryMsg::SimulateSwapOperations {
        offer_amount: Uint128::from(1000000u128),
        operations: vec![],
    };

    let error = query(deps.as_ref(), mock_env(), empty_operations_msg).unwrap_err();

    match error {
        ContractError::NoSwapOperationsProvided {} => (),
        _ => panic!("should return ContractError::NoSwapOperationsProvided"),
    }

    let msg = QueryMsg::SimulateSwapOperations {
        offer_amount: Uint128::from(1000000u128),
        operations: vec![
            SwapOperation::TerraSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: "ukrw".to_string(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: "asset0000".to_string(),
                },
            },
            SwapOperation::TerraSwap {
                offer_asset_info: AssetInfo::Token {
                    contract_addr: "asset0000".to_string(),
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
            },
        ],
    };

    deps.querier.with_pool_factory(
        &[
            (
                &"ukrwasset0000".to_string(),
                &PairInfo {
                    asset_infos: [
                        AssetInfo::NativeToken {
                            denom: "ukrw".to_string(),
                        },
                        AssetInfo::Token {
                            contract_addr: "asset0000".to_string(),
                        },
                    ],
                    contract_addr: "pair0000".to_string(),
                    liquidity_token: "liquidity0000".to_string(),
                    asset_decimals: [6u8, 6u8],
                    pair_type: PairType::ConstantProduct,
                },
            ),
            (
                &"asset0000uluna".to_string(),
                &PairInfo {
                    asset_infos: [
                        AssetInfo::Token {
                            contract_addr: "asset0000".to_string(),
                        },
                        AssetInfo::NativeToken {
                            denom: "uluna".to_string(),
                        },
                    ],
                    contract_addr: "pair0001".to_string(),
                    liquidity_token: "liquidity0001".to_string(),
                    asset_decimals: [6u8, 6u8],
                    pair_type: PairType::ConstantProduct,
                },
            ),
        ],
        &[("ukrw".to_string(), 6u8), ("uluna".to_string(), 6u8)],
    );

    let res: SimulateSwapOperationsResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(
        res,
        SimulateSwapOperationsResponse {
            amount: Uint128::from(1000000u128)
        }
    );
}

#[test]
fn query_reverse_routes_with_from_native() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        terraswap_factory: "terraswapfactory".to_string(),
    };

    let target_amount = 1000000u128;

    let info = mock_info("addr0000", &[coin(10000000, "ukrw")]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    deps.querier.with_balance(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        [Coin {
            amount: Uint128::from(1000000u128),
            denom: "ukrw".to_string(),
        }]
        .to_vec(),
    )]);

    deps.querier.with_token_balances(&[(
        &"asset0001".to_string(),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(1000000u128))],
    )]);

    // try simulating with empty operations
    let empty_operations_msg = QueryMsg::ReverseSimulateSwapOperations {
        ask_amount: Uint128::from(target_amount),
        operations: vec![],
    };

    let error = query(deps.as_ref(), mock_env(), empty_operations_msg).unwrap_err();

    match error {
        ContractError::NoSwapOperationsProvided {} => (),
        _ => panic!("should return ContractError::NoSwapOperationsProvided"),
    }

    let msg = QueryMsg::ReverseSimulateSwapOperations {
        ask_amount: Uint128::from(target_amount),
        operations: vec![SwapOperation::TerraSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: "ukrw".to_string(),
            },
            ask_asset_info: AssetInfo::Token {
                contract_addr: "asset0000".to_string(),
            },
        }],
    };

    deps.querier.with_pool_factory(
        &[
            (
                &"ukrwasset0000".to_string(),
                &PairInfo {
                    contract_addr: "pair0000".to_string(),
                    liquidity_token: "liquidity0000".to_string(),
                    asset_infos: [
                        AssetInfo::Token {
                            contract_addr: "asset0000".to_string(),
                        },
                        AssetInfo::NativeToken {
                            denom: "ukrw".to_string(),
                        },
                    ],
                    asset_decimals: [8u8, 6u8],
                    pair_type: PairType::ConstantProduct,
                },
            ),
            (
                &"asset0000uluna".to_string(),
                &PairInfo {
                    contract_addr: "pair0001".to_string(),
                    liquidity_token: "liquidity0001".to_string(),
                    asset_infos: [
                        AssetInfo::Token {
                            contract_addr: "asset0000".to_string(),
                        },
                        AssetInfo::NativeToken {
                            denom: "uluna".to_string(),
                        },
                    ],
                    asset_decimals: [8u8, 6u8],
                    pair_type: PairType::ConstantProduct,
                },
            ),
        ],
        &[("ukrw".to_string(), 6u8), ("uluna".to_string(), 6u8)],
    );

    let res: SimulateSwapOperationsResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();

    assert_eq!(
        res,
        SimulateSwapOperationsResponse {
            amount: Uint128::from(1000000u128),
        }
    );

    let offer_amount = res.amount;

    let msg = ExecuteMsg::ExecuteSwapOperation {
        operation: SwapOperation::TerraSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: "ukrw".to_string(),
            },
            ask_asset_info: AssetInfo::Token {
                contract_addr: "asset0000".to_string(),
            },
        },
        to: None,
    };
    let info = mock_info("addr0", &[coin(offer_amount.u128(), "ukrw")]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    match res {
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "pair0000".to_string(),
            funds: vec![coin(target_amount, "ukrw")],
            msg: to_binary(&PairExecuteMsg::Swap {
                offer_asset: Asset {
                    info: AssetInfo::NativeToken {
                        denom: "ukrw".to_string(),
                    },
                    amount: Uint128::from(target_amount),
                },
                belief_price: None,
                max_spread: None,
                to: None,
            })
            .unwrap(),
        })),],
    );
}

#[test]
fn query_reverse_routes_with_to_native() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        terraswap_factory: "terraswapfactory".to_string(),
    };

    let target_amount = 1000000u128;

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    deps.querier.with_token_balances(&[
        (
            &"asset0000".to_string(),
            &[(&"pair0000".to_string(), &Uint128::from(1000000u128))],
        ),
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(1000000u128))],
        ),
    ]);

    let msg = QueryMsg::ReverseSimulateSwapOperations {
        ask_amount: Uint128::from(target_amount),
        operations: vec![SwapOperation::TerraSwap {
            offer_asset_info: AssetInfo::Token {
                contract_addr: "asset0000".to_string(),
            },
            ask_asset_info: AssetInfo::NativeToken {
                denom: "ukrw".to_string(),
            },
        }],
    };

    deps.querier.with_pool_factory(
        &[
            (
                &"ukrwasset0000".to_string(),
                &PairInfo {
                    contract_addr: "pair0000".to_string(),
                    liquidity_token: "liquidity0000".to_string(),
                    asset_infos: [
                        AssetInfo::Token {
                            contract_addr: "asset0000".to_string(),
                        },
                        AssetInfo::NativeToken {
                            denom: "ukrw".to_string(),
                        },
                    ],
                    asset_decimals: [8u8, 6u8],
                    pair_type: PairType::ConstantProduct,
                },
            ),
            (
                &"asset0000uluna".to_string(),
                &PairInfo {
                    contract_addr: "pair0001".to_string(),
                    liquidity_token: "liquidity0001".to_string(),
                    asset_infos: [
                        AssetInfo::Token {
                            contract_addr: "asset0000".to_string(),
                        },
                        AssetInfo::NativeToken {
                            denom: "uluna".to_string(),
                        },
                    ],
                    asset_decimals: [8u8, 6u8],
                    pair_type: PairType::ConstantProduct,
                },
            ),
        ],
        &[("ukrw".to_string(), 6u8), ("uluna".to_string(), 6u8)],
    );

    let res: SimulateSwapOperationsResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();

    assert_eq!(
        res,
        SimulateSwapOperationsResponse {
            amount: Uint128::from(target_amount),
        }
    );

    let offer_amount = res.amount;

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0".to_string(),
        amount: offer_amount,
        msg: to_binary(&Cw20HookMsg::ExecuteSwapOperations {
            operations: vec![SwapOperation::TerraSwap {
                offer_asset_info: AssetInfo::Token {
                    contract_addr: "asset0000".to_string(),
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: "ukrw".to_string(),
                },
            }],
            minimum_receive: None,
            to: None,
        })
        .unwrap(),
    });
    let info = mock_info("addr0", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: MOCK_CONTRACT_ADDR.to_string(),
            funds: vec![],
            msg: to_binary(&ExecuteMsg::ExecuteSwapOperation {
                operation: SwapOperation::TerraSwap {
                    offer_asset_info: AssetInfo::Token {
                        contract_addr: "asset0000".to_string(),
                    },
                    ask_asset_info: AssetInfo::NativeToken {
                        denom: "ukrw".to_string(),
                    },
                },
                to: Some("addr0".to_string()),
            })
            .unwrap(),
        })),],
    );

    let msg = ExecuteMsg::ExecuteSwapOperation {
        operation: SwapOperation::TerraSwap {
            offer_asset_info: AssetInfo::Token {
                contract_addr: "asset0000".to_string(),
            },
            ask_asset_info: AssetInfo::NativeToken {
                denom: "ukrw".to_string(),
            },
        },
        to: None,
    };

    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset0000".to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: "pair0000".to_string(),
                amount: Uint128::from(target_amount),
                msg: to_binary(&pool_network::pair::Cw20HookMsg::Swap {
                    belief_price: None,
                    max_spread: None,
                    to: None,
                })
                .unwrap(),
            })
            .unwrap(),
        }))],
    );
}

#[test]
fn assert_minimum_receive_native_token() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_balance(&[(
        &"addr0000".to_string(),
        [Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }]
        .to_vec(),
    )]);

    let info = mock_info("addr0000", &[]);
    // success
    let msg = ExecuteMsg::AssertMinimumReceive {
        asset_info: AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        prev_balance: Uint128::zero(),
        minimum_receive: Uint128::from(1000000u128),
        receiver: "addr0000".to_string(),
    };
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // assertion failed; native token
    let msg = ExecuteMsg::AssertMinimumReceive {
        asset_info: AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        prev_balance: Uint128::zero(),
        minimum_receive: Uint128::from(1000001u128),
        receiver: "addr0000".to_string(),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(ContractError::MinimumReceiveAssertion {
            minimum_receive,
            swap_amount,
        }) => {
            assert_eq!(minimum_receive, Uint128::new(1000001));
            assert_eq!(swap_amount, Uint128::new(1000000));
        }
        _ => panic!("DO NOT ENTER HERE"),
    }
}

#[test]
fn assert_minimum_receive_token() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_token_balances(&[(
        &"token0000".to_string(),
        &[(&"addr0000".to_string(), &Uint128::from(1000000u128))],
    )]);

    let info = mock_info("addr0000", &[]);
    // success
    let msg = ExecuteMsg::AssertMinimumReceive {
        asset_info: AssetInfo::Token {
            contract_addr: "token0000".to_string(),
        },
        prev_balance: Uint128::zero(),
        minimum_receive: Uint128::from(1000000u128),
        receiver: "addr0000".to_string(),
    };
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // assertion failed; native token
    let msg = ExecuteMsg::AssertMinimumReceive {
        asset_info: AssetInfo::Token {
            contract_addr: "token0000".to_string(),
        },
        prev_balance: Uint128::zero(),
        minimum_receive: Uint128::from(1000001u128),
        receiver: "addr0000".to_string(),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(ContractError::MinimumReceiveAssertion {
            minimum_receive,
            swap_amount,
        }) => {
            assert_eq!(minimum_receive, Uint128::new(1000001));
            assert_eq!(swap_amount, Uint128::new(1000000));
        }
        _ => panic!("DO NOT ENTER HERE"),
    }
}

#[test]
fn can_migrate_contract() {
    let mut deps = mock_dependencies(&[]);
    let msg = InstantiateMsg {
        terraswap_factory: "terraswap_factory".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let res = migrate(deps.as_mut(), mock_env(), MigrateMsg {});

    // should not be able to migrate as the version is lower
    match res {
        Err(ContractError::MigrateInvalidVersion { .. }) => (),
        _ => panic!("should return ContractError::MigrateInvalidVersion"),
    }
}

#[test]
fn add_swap_routes() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        terraswap_factory: "terraswapfactory".to_string(),
    };

    let info = mock_info("creator", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    deps.querier.with_pool_factory(
        &[
            (
                &"ukrwasset0000".to_string(),
                &PairInfo {
                    asset_infos: [
                        AssetInfo::NativeToken {
                            denom: "ukrw".to_string(),
                        },
                        AssetInfo::Token {
                            contract_addr: "asset0000".to_string(),
                        },
                    ],
                    contract_addr: "pair0000".to_string(),
                    liquidity_token: "liquidity0000".to_string(),
                    asset_decimals: [6u8, 6u8],
                    pair_type: PairType::ConstantProduct,
                },
            ),
            (
                &"asset0000uluna".to_string(),
                &PairInfo {
                    asset_infos: [
                        AssetInfo::Token {
                            contract_addr: "asset0000".to_string(),
                        },
                        AssetInfo::NativeToken {
                            denom: "uluna".to_string(),
                        },
                    ],
                    contract_addr: "pair0001".to_string(),
                    liquidity_token: "liquidity0001".to_string(),
                    asset_decimals: [6u8, 6u8],
                    pair_type: PairType::ConstantProduct,
                },
            ),
            (
                &"ulunauwhale".to_string(),
                &PairInfo {
                    asset_infos: [
                        AssetInfo::NativeToken {
                            denom: "uluna".to_string(),
                        },
                        AssetInfo::NativeToken {
                            denom: "uwhale".to_string(),
                        },
                    ],
                    contract_addr: "pair0002".to_string(),
                    liquidity_token: "liquidity0002".to_string(),
                    asset_decimals: [6u8, 6u8],
                    pair_type: PairType::ConstantProduct,
                },
            ),
        ],
        &[
            ("ukrw".to_string(), 6u8),
            ("uluna".to_string(), 6u8),
            ("uwhale".to_string(), 6u8),
        ],
    );

    let swap_route_1 = SwapRoute {
        offer_asset_info: AssetInfo::NativeToken {
            denom: "ukrw".to_string(),
        },
        ask_asset_info: AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        },
        swap_operations: vec![
            SwapOperation::TerraSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: "ukrw".to_string(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: "asset0000".to_string(),
                },
            },
            SwapOperation::TerraSwap {
                offer_asset_info: AssetInfo::Token {
                    contract_addr: "asset0000".to_string(),
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
            },
        ],
    };

    let swap_route_2 = SwapRoute {
        offer_asset_info: AssetInfo::NativeToken {
            denom: "ukrw".to_string(),
        },
        ask_asset_info: AssetInfo::NativeToken {
            denom: "uwhale".to_string(),
        },
        swap_operations: vec![
            SwapOperation::TerraSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: "ukrw".to_string(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: "asset0000".to_string(),
                },
            },
            SwapOperation::TerraSwap {
                offer_asset_info: AssetInfo::Token {
                    contract_addr: "asset0000".to_string(),
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
            },
            SwapOperation::TerraSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
            },
        ],
    };

    let swap_route_1_key = SWAP_ROUTES.key((
        swap_route_1
            .clone()
            .offer_asset_info
            .get_label(&deps.as_ref())
            .unwrap()
            .as_str(),
        swap_route_1
            .clone()
            .ask_asset_info
            .get_label(&deps.as_ref())
            .unwrap()
            .as_str(),
    ));

    let swap_route_2_key = SWAP_ROUTES.key((
        swap_route_2
            .clone()
            .offer_asset_info
            .get_label(&deps.as_ref())
            .unwrap()
            .as_str(),
        swap_route_2
            .clone()
            .ask_asset_info
            .get_label(&deps.as_ref())
            .unwrap()
            .as_str(),
    ));

    // verify the keys are not there
    assert_eq!(None, swap_route_1_key.may_load(&deps.storage).unwrap());
    assert_eq!(None, swap_route_2_key.may_load(&deps.storage).unwrap());

    // add swap route
    let msg = ExecuteMsg::AddSwapRoutes {
        swap_routes: vec![swap_route_1.clone(), swap_route_2.clone()],
    };

    let res = execute(deps.as_mut(), mock_env(), mock_info("creator", &[]), msg).unwrap();
    let expected_attributes = vec![
        attr("action", "add_swap_routes"),
        attr("swap_route", swap_route_1.to_string()),
        attr("swap_route", swap_route_2.to_string()),
    ];

    assert_eq!(res.messages.len(), 0usize);
    assert_eq!(res.attributes, expected_attributes);

    // query swap route
    let msg = QueryMsg::SwapRoute {
        offer_asset_info: swap_route_1.offer_asset_info.clone(),
        ask_asset_info: swap_route_1.ask_asset_info.clone(),
    };

    let res: Vec<SwapOperation> =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(res, swap_route_1.swap_operations);
}

#[test]
fn add_swap_routes_invalid_route() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        terraswap_factory: "terraswapfactory".to_string(),
    };

    let info = mock_info("creator", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    deps.querier.with_pool_factory(
        &[
            (
                &"ukrwasset0000".to_string(),
                &PairInfo {
                    asset_infos: [
                        AssetInfo::NativeToken {
                            denom: "ukrw".to_string(),
                        },
                        AssetInfo::Token {
                            contract_addr: "asset0000".to_string(),
                        },
                    ],
                    contract_addr: "pair0000".to_string(),
                    liquidity_token: "liquidity0000".to_string(),
                    asset_decimals: [6u8, 6u8],
                    pair_type: PairType::ConstantProduct,
                },
            ),
            (
                &"ulunauwhale".to_string(),
                &PairInfo {
                    asset_infos: [
                        AssetInfo::NativeToken {
                            denom: "uluna".to_string(),
                        },
                        AssetInfo::NativeToken {
                            denom: "uwhale".to_string(),
                        },
                    ],
                    contract_addr: "pair0002".to_string(),
                    liquidity_token: "liquidity0002".to_string(),
                    asset_decimals: [6u8, 6u8],
                    pair_type: PairType::ConstantProduct,
                },
            ),
        ],
        &[
            ("ukrw".to_string(), 6u8),
            ("uluna".to_string(), 6u8),
            ("uwhale".to_string(), 6u8),
        ],
    );

    let offer_asset_info = AssetInfo::NativeToken {
        denom: "ukrw".to_string(),
    };

    let ask_asset_info = AssetInfo::NativeToken {
        denom: "uluna".to_string(),
    };

    let swap_route_1 = SwapRoute {
        offer_asset_info: offer_asset_info.clone(),
        ask_asset_info: ask_asset_info.clone(),
        swap_operations: vec![
            SwapOperation::TerraSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: "ukrw".to_string(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: "asset0000".to_string(),
                },
            },
            SwapOperation::TerraSwap {
                offer_asset_info: AssetInfo::Token {
                    contract_addr: "asset0000".to_string(),
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
            },
        ],
    };

    // add swap route
    let msg = ExecuteMsg::AddSwapRoutes {
        swap_routes: vec![swap_route_1.clone()],
    };

    let error = execute(deps.as_mut(), mock_env(), mock_info("creator", &[]), msg).unwrap_err();

    match error {
        ContractError::InvalidSwapRoute(swap_route) => assert_eq!(swap_route, swap_route_1),
        _ => panic!("should return ContractError::InvalidSwapRoute"),
    }

    // query swap route
    let msg = QueryMsg::SwapRoute {
        offer_asset_info: offer_asset_info.clone(),
        ask_asset_info: ask_asset_info.clone(),
    };

    let error = query(deps.as_ref(), mock_env(), msg).unwrap_err();

    match error {
        ContractError::NoSwapRouteForAssets {
            offer_asset,
            ask_asset,
        } => {
            assert_eq!(offer_asset_info.to_string(), offer_asset);
            assert_eq!(ask_asset_info.to_string(), ask_asset);
        }
        _ => panic!("should return ContractError::NoSwapRouteForAssets"),
    }
}

#[test]
fn add_swap_routes_unauthorized() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        terraswap_factory: "terraswapfactory".to_string(),
    };

    let info = mock_info("creator", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let offer_asset_info = AssetInfo::NativeToken {
        denom: "ukrw".to_string(),
    };

    let ask_asset_info = AssetInfo::NativeToken {
        denom: "uluna".to_string(),
    };

    let swap_route_1 = SwapRoute {
        offer_asset_info: offer_asset_info.clone(),
        ask_asset_info: ask_asset_info.clone(),
        swap_operations: vec![
            SwapOperation::TerraSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: "ukrw".to_string(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: "asset0000".to_string(),
                },
            },
            SwapOperation::TerraSwap {
                offer_asset_info: AssetInfo::Token {
                    contract_addr: "asset0000".to_string(),
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
            },
        ],
    };

    // add swap route
    let msg = ExecuteMsg::AddSwapRoutes {
        swap_routes: vec![swap_route_1.clone()],
    };

    let error = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("unauthorized", &[]),
        msg,
    )
    .unwrap_err();

    match error {
        ContractError::Unauthorized {} => (),
        _ => panic!("should return ContractError::Unauthorized"),
    }
}
