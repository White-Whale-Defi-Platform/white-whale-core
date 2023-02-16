use crate::contract::{execute, instantiate};
use crate::error::ContractError;
use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{to_binary, Coin, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;
use pool_network::asset::{Asset, AssetInfo, PairType};
use pool_network::mock_querier::mock_dependencies;
use pool_network::pair::ExecuteMsg::UpdateConfig;
use pool_network::pair::{Cw20HookMsg, ExecuteMsg, FeatureToggle, InstantiateMsg, PoolFee};
use white_whale::fee::Fee;

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
    instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // all features are enabled by default, let's disable swaps
    let update_config_message = UpdateConfig {
        owner: None,
        fee_collector_addr: None,
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
            burn_fee: Fee {
                share: Decimal::zero(),
            },
        },
        fee_collector_addr: "collector".to_string(),
        pair_type: PairType::ConstantProduct,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // all features are enabled by default, let's disable withdrawals
    let update_config_message = UpdateConfig {
        owner: None,
        fee_collector_addr: None,
        pool_fees: None,
        feature_toggle: Some(FeatureToggle {
            withdrawals_enabled: false,
            deposits_enabled: true,
            swaps_enabled: true,
        }),
    };
    execute(deps.as_mut(), env, info, update_config_message).unwrap();

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
            burn_fee: Fee {
                share: Decimal::zero(),
            },
        },
        fee_collector_addr: "collector".to_string(),
        pair_type: PairType::ConstantProduct,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // all features are enabled by default, let's disable deposits
    let update_config_message = UpdateConfig {
        owner: None,
        fee_collector_addr: None,
        pool_fees: None,
        feature_toggle: Some(FeatureToggle {
            withdrawals_enabled: true,
            deposits_enabled: false,
            swaps_enabled: true,
        }),
    };
    execute(deps.as_mut(), env, info, update_config_message).unwrap();

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
