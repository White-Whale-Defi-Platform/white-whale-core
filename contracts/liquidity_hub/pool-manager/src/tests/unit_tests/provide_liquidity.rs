#[cfg(feature = "token_factory")]
use crate::state::LP_SYMBOL;
use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    attr, to_binary, Coin, CosmosMsg, Decimal, Reply, Response, StdError, SubMsg, SubMsgResponse,
    SubMsgResult, Uint128, WasmMsg,
};
#[cfg(feature = "token_factory")]
use cosmwasm_std::{coin, BankMsg};
use cw20::Cw20ExecuteMsg;
use white_whale_std::fee::Fee;

use crate::tests::mock_querier::mock_dependencies;
#[cfg(feature = "token_factory")]
use white_whale_std::pool_network;
use white_whale_std::pool_network::asset::{Asset, AssetInfo, PairType, MINIMUM_LIQUIDITY_AMOUNT};
#[cfg(feature = "token_factory")]
use white_whale_std::pool_network::denom::MsgMint;
use white_whale_std::pool_network::pair::PoolFee;
// use white_whale_std::pool_network::pair::{ExecuteMsg, InstantiateMsg, PoolFee};
use crate::contract::{execute, instantiate};
use crate::error::ContractError;
use white_whale_std::pool_manager::ExecuteMsg;
use white_whale_std::pool_manager::InstantiateMsg as SingleSwapInstantiateMsg;
#[test]
fn provide_liquidity_cw20_lp() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(2_000u128),
    }]);

    // Note: In order to support and write tests sooner I override the MockAPI in mock_querier with a SimpleMockAPI
    // The effect is I can continue testing with instantiate2 but now an addr like liquidity0000 becomes TEMP_CONTRACT_ADDR
    // This likely breaks tests elsewhere
    let TEMP_CONTRACT_ADDR: String =
        "4j®Q¶õ\u{1c}¼\u{f}º\u{8d}N\u{7}SÈ¶ö\u{7}©³8\u{7f}j¼Þp\u{9c}\u{1b}æù\u{a0}î".to_string();

    deps.querier.with_token_balances(&[
        (
            &TEMP_CONTRACT_ADDR.clone(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::zero())],
        ),
        (
            &"asset0000".to_string(),
            &[(&"addr0000".to_string(), &Uint128::from(100u128))],
        ),
    ]);
    deps.querier
        .with_pool_factory(&[], &[("uusd".to_string(), 6u8)]);

    // Instantiate contract
    let msg = SingleSwapInstantiateMsg {
        fee_collector_addr: "fee_collector_addr".to_string(),
        owner: "owner".to_string(),
        pair_code_id: 10u64,
        token_code_id: 11u64,
        pool_creation_fee: Asset {
            amount: Uint128::new(1000000u128),
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
        },
    };
    let env = mock_env();
    let info = mock_info("owner", &[]);
    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // Create the Pair
    let asset_infos = [
        AssetInfo::Token {
            contract_addr: "asset0000".to_string(),
        },
        AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
    ];

    let msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos.to_vec(),
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
        pair_type: PairType::ConstantProduct,
        token_factory_lp: false,
        pair_identifier: None,
    };

    let env = mock_env();

    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(1000000u128),
        }],
    );

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

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
        ]
        .to_vec(),
        slippage_tolerance: None,
        receiver: None,
        pair_identifier: "0".to_string(),
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
        _ => {
            println!("{:?}", res);
            panic!("should return ContractError::InvalidInitialLiquidityAmount")
        }
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
        ]
        .to_vec(),
        slippage_tolerance: None,
        receiver: None,
        pair_identifier: "0".to_string(),
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

    assert_eq!(res.messages.len(), 2usize);

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
            contract_addr: TEMP_CONTRACT_ADDR.clone(),
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
            contract_addr: TEMP_CONTRACT_ADDR.clone(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "addr0000".to_string(),
                amount: MINIMUM_LIQUIDITY_AMOUNT,
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
            &TEMP_CONTRACT_ADDR.to_string(),
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
        ]
        .to_vec(),
        slippage_tolerance: None,
        receiver: Some("staking0000".to_string()), // try changing receiver
        pair_identifier: 1.to_string(),
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
    assert_eq!(res.messages.len(), 3usize);

    let transfer_from_msg = res.messages.get(0).expect("no message");
    let mint_msg = res.messages.get(1).expect("no message");
    assert_eq!(
        transfer_from_msg,
        &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset0000".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                owner: "addr0000".to_string(),
                recipient: MOCK_CONTRACT_ADDR.to_string(),
                amount: Uint128::from(200u128),
            })
            .unwrap(),
            funds: vec![],
        }))
    );
    assert_eq!(
        mint_msg,
        &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: TEMP_CONTRACT_ADDR.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: "cosmos2contract".to_string(), // LP tokens sent to specified receiver
                amount: Uint128::from(33u128),
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
        ]
        .to_vec(),
        slippage_tolerance: None,
        receiver: None,
        pair_identifier: 1.to_string(),
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
        _ => {
            panic!("Must return generic error");
        }
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
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(200u128))],
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
        ]
        .to_vec(),
        slippage_tolerance: Some(Decimal::percent(1)),
        receiver: None,
        pair_identifier: 1.to_string(),
    };

    let env = mock_env();
    let info = mock_info(
        "addr0001",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );

    deps.querier.with_token_balances(&[
        (
            &TEMP_CONTRACT_ADDR.to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(100u128))],
        ),
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(100u128))],
        ),
    ]);
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
        ]
        .to_vec(),
        slippage_tolerance: Some(Decimal::percent(1)),
        receiver: None,
        pair_identifier: 1.to_string(),
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

    deps.querier.with_token_balances(&[
        (
            &TEMP_CONTRACT_ADDR.clone(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(100u128))],
        ),
        (
            &"asset0000".to_string(),
            &[
                (&"addr0001".to_string(), &Uint128::from(100u128)),
                (&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(100u128)),
            ],
        ),
    ]);

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
        ]
        .to_vec(),
        slippage_tolerance: Some(Decimal::percent(2)),
        receiver: None,
        pair_identifier: 1.to_string(),
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
        ]
        .to_vec(),
        slippage_tolerance: Some(Decimal::percent(2)),
        receiver: None,
        pair_identifier: 1.to_string(),
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
        amount: Uint128::from(2_000u128),
    }]);

    // Note: In order to support and write tests sooner I override the MockAPI in mock_querier with a SimpleMockAPI
    // The effect is I can continue testing with instantiate2 but now an addr like liquidity0000 becomes TEMP_CONTRACT_ADDR
    // This likely breaks tests elsewhere
    let TEMP_CONTRACT_ADDR: String =
        "contract757573642d6d4141504c61646472303030303132333435".to_string();

    deps.querier.with_token_balances(&[
        (
            &TEMP_CONTRACT_ADDR.clone(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::zero())],
        ),
        (&"asset0000".to_string(), &[]),
    ]);

    deps.querier
        .with_pool_factory(&[], &[("uusd".to_string(), 6u8)]);

    // Instantiate contract
    let msg = SingleSwapInstantiateMsg {
        fee_collector_addr: "fee_collector_addr".to_string(),
        owner: "owner".to_string(),
        pair_code_id: 10u64,
        token_code_id: 11u64,
        pool_creation_fee: Asset {
            amount: Uint128::new(1000000u128),
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
        },
    };
    let env = mock_env();
    let info = mock_info("owner", &[]);
    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // Create the Pair
    let asset_infos = [
        AssetInfo::Token {
            contract_addr: "asset0000".to_string(),
        },
        AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
    ];

    let msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos.to_vec(),
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
        pair_type: PairType::ConstantProduct,
        token_factory_lp: false,
        pair_identifier: None,
    };

    let env = mock_env();

    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(1000000u128),
        }],
    );

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Instantiate contract
    let msg = SingleSwapInstantiateMsg {
        fee_collector_addr: "fee_collector_addr".to_string(),
        owner: "owner".to_string(),
        pair_code_id: 10u64,
        token_code_id: 11u64,
        pool_creation_fee: Asset {
            amount: Uint128::new(1000000u128),
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
        },
    };
    let env = mock_env();
    let info = mock_info("owner", &[]);
    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());

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
        ]
        .to_vec(),
        slippage_tolerance: None,
        receiver: None,
        pair_identifier: "0".to_string(),
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
        _ => {
            panic!("should return ContractError::InvalidZeroAmount")
        }
    }
}

#[test]
fn provide_liquidity_invalid_minimum_lp_amount() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(999u128),
    }]);

    // Note: In order to support and write tests sooner I override the MockAPI in mock_querier with a SimpleMockAPI
    // The effect is I can continue testing with instantiate2 but now an addr like liquidity0000 becomes TEMP_CONTRACT_ADDR
    // This likely breaks tests elsewhere
    let TEMP_CONTRACT_ADDR: String =
        "contract6d4141504c2d7575736461646472303030303132333435".to_string();

    deps.querier.with_token_balances(&[
        (
            &TEMP_CONTRACT_ADDR.to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::zero())],
        ),
        (&"asset0001".to_string(), &[]),
    ]);
    deps.querier
        .with_pool_factory(&[], &[("uusd".to_string(), 6u8)]);

    // Instantiate contract
    let msg = SingleSwapInstantiateMsg {
        fee_collector_addr: "fee_collector_addr".to_string(),
        owner: "owner".to_string(),
        pair_code_id: 10u64,
        token_code_id: 11u64,
        pool_creation_fee: Asset {
            amount: Uint128::new(1000000u128),
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
        },
    };
    let env = mock_env();
    let info = mock_info("owner", &[]);
    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    let asset_infos = [
        AssetInfo::Token {
            contract_addr: "asset0001".to_string(),
        },
        AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
    ];

    let msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos.to_vec(),
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
        pair_type: PairType::ConstantProduct,
        token_factory_lp: false,
        pair_identifier: None,
    };

    let env = mock_env();
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(1000000u128),
        }],
    );
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "create_pair"),
            attr("pair", "mAAPL-uusd"),
            attr("pair_label", "mAAPL-uusd"),
            attr("pair_type", "ConstantProduct"),
        ]
    );

    // successfully provide liquidity for the exist pool
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: "asset0001".to_string(),
                },
                amount: (MINIMUM_LIQUIDITY_AMOUNT - Uint128::one()),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(100u128),
            },
        ]
        .to_vec(),
        slippage_tolerance: None,
        receiver: None,
        pair_identifier: 1.to_string(),
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
        e => {
            println!("{:?}", e.unwrap());
            panic!("should return ContractError::InvalidInitialLiquidityAmount");
        }
    }
}

#[cfg(feature = "token_factory")]
#[test]
fn provide_liquidity_tokenfactory_lp() {
    let lp_denom = format!("{}/{MOCK_CONTRACT_ADDR}/{LP_SYMBOL}", "factory");

    let mut deps = mock_dependencies(&[
        Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(2_000u128),
        },
        Coin {
            denom: "uwhale".to_string(),
            amount: Uint128::from(2_000u128),
        },
        Coin {
            denom: lp_denom.clone(),
            amount: Uint128::zero(),
        },
    ]);

    deps.querier
        .with_token_balances(&[(&"asset0000".to_string(), &[])]);

    // Instantiate contract
    let msg = SingleSwapInstantiateMsg {
        fee_collector_addr: "fee_collector_addr".to_string(),
        owner: "owner".to_string(),
        pair_code_id: 10u64,
        token_code_id: 11u64,
        pool_creation_fee: vec![Asset {
            amount: Uint128::new(1000000u128),
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
        }],
    };
    let env = mock_env();
    let info = mock_info("owner", &[]);
    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    // successfully provide liquidity for the exist pool
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::from(2_000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(2_000u128),
            },
        ]
        .to_vec(),
        slippage_tolerance: None,
        receiver: None,
    };

    let env = mock_env();
    let info = mock_info(
        "addr0000",
        &[
            Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(2_000u128),
            },
            Coin {
                denom: "uwhale".to_string(),
                amount: Uint128::from(2_000u128),
            },
        ],
    );
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(res.messages.len(), 3usize);

    let mint_initial_lp_msg = res.messages.get(0).expect("no message").clone().msg;
    let mint_msg = res.messages.get(1).expect("no message").clone().msg;
    let bank_send_msg = res.messages.get(2).expect("no message").clone().msg;

    let mint_initial_lp_msg_expected = <MsgMint as Into<CosmosMsg>>::into(MsgMint {
        sender: MOCK_CONTRACT_ADDR.to_string(),
        amount: Some(pool_network::denom::Coin {
            denom: lp_denom.clone(),
            amount: MINIMUM_LIQUIDITY_AMOUNT.to_string(),
        }),
    });

    let mint_msg_expected = <MsgMint as Into<CosmosMsg>>::into(MsgMint {
        sender: MOCK_CONTRACT_ADDR.to_string(),
        amount: Some(pool_network::denom::Coin {
            denom: lp_denom.clone(),
            amount: "1000".to_string(),
        }),
    });

    let bank_send_msg_expected = CosmosMsg::Bank(BankMsg::Send {
        to_address: "addr0000".to_string(),
        amount: vec![coin(1000u128, lp_denom.clone())],
    });

    assert_eq!(mint_initial_lp_msg, mint_initial_lp_msg_expected);

    assert_eq!(mint_msg, mint_msg_expected);

    assert_eq!(bank_send_msg, bank_send_msg_expected);
}