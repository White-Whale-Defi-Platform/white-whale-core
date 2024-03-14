use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_json, to_json_binary, Addr, Decimal, Reply, ReplyOn, StdError, SubMsg, SubMsgResponse,
    SubMsgResult, Uint128, WasmMsg,
};
use cw20::MinterResponse;

#[cfg(feature = "token_factory")]
use crate::state::LP_SYMBOL;
#[cfg(feature = "token_factory")]
use cosmwasm_std::CosmosMsg;
use white_whale_std::fee::Fee;
use white_whale_std::pool_network::asset::{Asset, AssetInfo, TrioInfo};
#[cfg(feature = "token_factory")]
use white_whale_std::pool_network::denom::MsgCreateDenom;
use white_whale_std::pool_network::mock_querier::mock_dependencies;
use white_whale_std::pool_network::swap::assert_max_spread;
use white_whale_std::pool_network::token::InstantiateMsg as TokenInstantiateMsg;
use white_whale_std::pool_network::trio::ExecuteMsg::UpdateConfig;
use white_whale_std::pool_network::trio::{Config, InstantiateMsg, MigrateMsg, PoolFee, QueryMsg};

use crate::contract::{execute, instantiate, migrate, query, reply};
use crate::error::ContractError;
use crate::helpers::assert_slippage_tolerance;
use crate::queries::query_trio_info;

#[cfg(not(feature = "osmosis"))]
#[test]
fn proper_initialization_cw20_lp() {
    let mut deps = mock_dependencies(&[]);

    deps.querier.with_token_balances(&[
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(123u128))],
        ),
        (
            &"asset0001".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(456u128))],
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
            AssetInfo::Token {
                contract_addr: "asset0001".to_string(),
            },
        ],
        token_code_id: 10u64,
        asset_decimals: [6u8, 8u8, 10u8],
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
        amp_factor: 1000,
        token_factory_lp: false,
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
                msg: to_json_binary(&TokenInstantiateMsg {
                    name: "uusd-mAAPL-mAAPL-LP".to_string(),
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
                label: "uusd-mAAPL-mAAPL-LP".to_string(),
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
    let trio_info: TrioInfo = query_trio_info(deps.as_ref()).unwrap();
    assert_eq!("liquidity0000", trio_info.liquidity_token.to_string());
    assert_eq!(
        trio_info.asset_infos,
        [
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            AssetInfo::Token {
                contract_addr: "asset0000".to_string()
            },
            AssetInfo::Token {
                contract_addr: "asset0001".to_string()
            }
        ]
    );
}

#[cfg(feature = "token_factory")]
#[test]
fn proper_initialization_tokenfactory_lp() {
    let mut deps = mock_dependencies(&[]);

    deps.querier.with_token_balances(&[
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(123u128))],
        ),
        (
            &"asset0001".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(456u128))],
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
            AssetInfo::Token {
                contract_addr: "asset0001".to_string(),
            },
        ],
        token_code_id: 10u64,
        asset_decimals: [6u8, 8u8, 10u8],
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
        amp_factor: 1000,
        token_factory_lp: true,
    };

    // we can just call .unwrap() to assert this was a success
    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    let expected = <MsgCreateDenom as Into<CosmosMsg>>::into(MsgCreateDenom {
        sender: MOCK_CONTRACT_ADDR.to_string(),
        subdenom: LP_SYMBOL.to_string(),
    });

    assert_eq!(res.messages[0].msg, expected);

    // let's query the state
    let trio_info: TrioInfo = query_trio_info(deps.as_ref()).unwrap();
    assert_eq!(
        trio_info.liquidity_token,
        AssetInfo::NativeToken {
            denom: format!("{}/{MOCK_CONTRACT_ADDR}/{LP_SYMBOL}", "factory")
        }
    );
}

#[cfg(not(feature = "osmosis"))]
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
            AssetInfo::Token {
                contract_addr: "asset0001".to_string(),
            },
        ],
        token_code_id: 10u64,
        asset_decimals: [6u8, 8u8, 10u8],
        pool_fees: PoolFee {
            protocol_fee: Fee {
                share: Decimal::from_ratio(Uint128::from(2u8), Uint128::from(1u8)),
            },
            swap_fee: Fee {
                share: Decimal::percent(1u64),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
        },
        fee_collector_addr: "collector".to_string(),
        amp_factor: 1000,
        token_factory_lp: false,
    };

    // we can just call .unwrap() to assert this was a success
    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let res = instantiate(deps.as_mut(), env, info, msg);
    match res {
        Ok(_) => panic!("should return StdError::generic_err(Invalid fee)"),
        Err(ContractError::Std { .. }) => (),
        _ => panic!("should return StdError::generic_err(Invalid fee)"),
    }
}

#[cfg(not(feature = "osmosis"))]
#[test]
fn test_initialization_invalid_amp() {
    let mut deps = mock_dependencies(&[]);

    deps.querier.with_token_balances(&[
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(123u128))],
        ),
        (
            &"asset0001".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(456u128))],
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
            AssetInfo::Token {
                contract_addr: "asset0001".to_string(),
            },
        ],
        token_code_id: 10u64,
        asset_decimals: [6u8, 8u8, 10u8],
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
        amp_factor: 0,
        token_factory_lp: false,
    };

    // we can just call .unwrap() to assert this was a success
    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let res = instantiate(deps.as_mut(), env, info, msg);
    match res {
        Ok(_) => panic!("should return ContractError::Std(Invalid amp)"),
        Err(ContractError::Std { .. }) => (),
        _ => panic!("should return ContractError::Std(Invalid amp)"),
    }
}

#[cfg(not(feature = "osmosis"))]
#[test]
fn can_migrate_contract() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_token_balances(&[
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(123u128))],
        ),
        (
            &"asset0001".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(456u128))],
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
            AssetInfo::Token {
                contract_addr: "asset0001".to_string(),
            },
        ],
        token_code_id: 10u64,
        asset_decimals: [6u8, 8u8, 10u8],
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
        amp_factor: 1000,
        token_factory_lp: false,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    instantiate(deps.as_mut(), env, info, msg).unwrap();

    let res = migrate(deps.as_mut(), mock_env(), MigrateMsg {});

    // should not be able to migrate as the version is lower
    match res {
        Err(ContractError::MigrateInvalidVersion { .. }) => (),
        _ => panic!("should return ContractError::MigrateInvalidVersion"),
    }
}

#[test]
fn test_max_spread() {
    assert_max_spread(
        Some(Decimal::from_ratio(1_200_000_000_u128, 1_000_000u128)),
        Some(Decimal::percent(1)),
        Uint128::from(1_200_000_000_u128),
        Uint128::from(989_999u128),
        Uint128::zero(),
    )
    .unwrap_err();

    // same example as above but using 6 and 18 decimal places
    assert_max_spread(
        Some(Decimal::from_ratio(
            1_200_000_000_u128,
            1_000_000_000_000_000_000u128,
        )),
        Some(Decimal::percent(1)),
        Uint128::from(1_200_000_000_u128),
        Uint128::from(989_999_900_000_000_000u128),
        Uint128::zero(),
    )
    .unwrap_err();

    assert_max_spread(
        Some(Decimal::from_ratio(1_200_000_000_u128, 1_000_000u128)),
        None, // defaults to 0.5%
        Uint128::from(1_200_000_000_u128),
        Uint128::from(995_000u128), // all good
        Uint128::zero(),
    )
    .unwrap();

    assert_max_spread(
        Some(Decimal::from_ratio(1_200_000_000_u128, 1_000_000u128)),
        None, // defaults to 0.1%
        Uint128::from(1_200_000_000_u128),
        Uint128::from(989_000u128), // fails
        Uint128::zero(),
    )
    .unwrap_err();

    assert_max_spread(
        Some(Decimal::from_ratio(1_200_000_000_u128, 1_000_000u128)),
        Some(Decimal::percent(1)),
        Uint128::from(1_200_000_000_u128),
        Uint128::from(990_000u128),
        Uint128::zero(),
    )
    .unwrap();

    // same example as above but using 6 and 18 decimal place
    assert_max_spread(
        Some(Decimal::from_ratio(
            1_200_000_000_u128,
            1_000_000_000_000_000_000u128,
        )),
        Some(Decimal::percent(1)),
        Uint128::from(1_200_000_000_u128),
        Uint128::from(990_000_000_000_000_000_u128),
        Uint128::zero(),
    )
    .unwrap();

    // similar example with 18 and 6 decimal places
    assert_max_spread(
        Some(Decimal::from_ratio(
            1_000_000_000_000_000_000u128,
            10_000_000u128,
        )),
        Some(Decimal::percent(2)),
        Uint128::from(1_000_000_000_000_000_000u128),
        Uint128::from(9_800_000u128),
        Uint128::zero(),
    )
    .unwrap();

    // same as before but error because spread is 1%
    assert_max_spread(
        Some(Decimal::from_ratio(
            1_000_000_000_000_000_000u128,
            10_000_000u128,
        )),
        Some(Decimal::percent(1)),
        Uint128::from(1_000_000_000_000_000_000u128),
        Uint128::from(9_800_000u128),
        Uint128::zero(),
    )
    .unwrap_err();

    assert_max_spread(
        None,
        Some(Decimal::percent(1)),
        Uint128::zero(),
        Uint128::from(989_999u128),
        Uint128::from(10001u128),
    )
    .unwrap_err();

    assert_max_spread(
        None,
        Some(Decimal::percent(1)),
        Uint128::zero(),
        Uint128::from(990_000u128),
        Uint128::from(10000u128),
    )
    .unwrap();

    assert_max_spread(
        Some(Decimal::from_ratio(1_200_000_000_u128, 1_000_000u128)),
        Some(Decimal::percent(60)), // this will default to 50%
        Uint128::from(1_200_000_000_u128),
        Uint128::from(989_999u128),
        Uint128::zero(),
    )
    .unwrap();
}

#[cfg(not(feature = "osmosis"))]
#[test]
fn test_update_config_unsuccessful() {
    let mut deps = mock_dependencies(&[]);

    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::zero())],
        ),
        (&"asset0000".to_string(), &[]),
        (&"asset0001".to_string(), &[]),
    ]);

    let msg = InstantiateMsg {
        asset_infos: [
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            AssetInfo::Token {
                contract_addr: "asset0000".to_string(),
            },
            AssetInfo::Token {
                contract_addr: "asset0001".to_string(),
            },
        ],
        token_code_id: 10u64,
        asset_decimals: [6u8, 8u8, 10u8],
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
        amp_factor: 1000,
        token_factory_lp: false,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    // we can just call .unwrap() to assert this was a success
    instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // update config with invalid fees
    let update_config_message = UpdateConfig {
        owner: None,
        fee_collector_addr: None,
        pool_fees: Some(PoolFee {
            protocol_fee: Fee {
                share: Decimal::MAX,
            },
            swap_fee: Fee {
                share: Decimal::percent(1u64),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
        }),
        feature_toggle: None,
        amp_factor: None,
    };

    let res = execute(deps.as_mut(), env.clone(), info, update_config_message);
    match res {
        Ok(_) => panic!("should return Std(GenericErr -> msg: Invalid fee)"),
        Err(ContractError::Std(e)) => assert_eq!(e, StdError::generic_err("Invalid fee")),
        _ => panic!("should return Std(GenericErr -> msg: Invalid fee)"),
    }

    // an unauthorized party tries to update the config
    let info = mock_info("unauthorized", &[]);
    let update_config_message = UpdateConfig {
        owner: Some("unauthorized".to_string()),
        fee_collector_addr: None,
        pool_fees: None,
        feature_toggle: None,
        amp_factor: None,
    };

    let res = execute(deps.as_mut(), env, info, update_config_message);
    match res {
        Ok(_) => panic!("should return Std(GenericErr -> msg: unauthorized)"),
        Err(ContractError::Std { .. }) => (),
        _ => panic!("should return Std(GenericErr -> msg: unauthorized)"),
    }
}

#[cfg(not(feature = "osmosis"))]
#[test]
fn test_update_config_successful() {
    let mut deps = mock_dependencies(&[]);

    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::zero())],
        ),
        (&"asset0000".to_string(), &[]),
        (&"asset0001".to_string(), &[]),
    ]);

    let msg = InstantiateMsg {
        asset_infos: [
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            AssetInfo::Token {
                contract_addr: "asset0000".to_string(),
            },
            AssetInfo::Token {
                contract_addr: "asset0001".to_string(),
            },
        ],
        token_code_id: 10u64,
        asset_decimals: [6u8, 8u8, 10u8],
        pool_fees: PoolFee {
            protocol_fee: Fee {
                share: Decimal::zero(),
            },
            swap_fee: Fee {
                share: Decimal::zero(),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
        },
        fee_collector_addr: "collector".to_string(),
        amp_factor: 1000,
        token_factory_lp: false,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    // we can just call .unwrap() to assert this was a success
    instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let config: Config =
        from_json(query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();

    // check for original config
    assert_eq!(config.owner, Addr::unchecked("addr0000"));
    assert!(config.feature_toggle.swaps_enabled);
    assert_eq!(config.pool_fees.swap_fee.share, Decimal::zero());

    let update_config_message = UpdateConfig {
        owner: Some("new_admin".to_string()),
        fee_collector_addr: Some("new_collector".to_string()),
        pool_fees: Some(PoolFee {
            protocol_fee: Fee {
                share: Decimal::percent(1u64),
            },
            swap_fee: Fee {
                share: Decimal::percent(3u64),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
        }),
        feature_toggle: None,
        amp_factor: None,
    };

    execute(deps.as_mut(), env, info, update_config_message).unwrap();

    let config: Config =
        from_json(query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();

    // check for new config
    assert_eq!(config.owner, Addr::unchecked("new_admin"));
    assert_eq!(config.fee_collector_addr, Addr::unchecked("new_collector"));
    assert_eq!(config.pool_fees.swap_fee.share, Decimal::percent(3u64));
}

#[test]
fn test_assert_slippage_tolerance_invalid_ratio() {
    let res = assert_slippage_tolerance(
        &Some(Decimal::from_ratio(Uint128::new(2), Uint128::new(1))),
        &[Uint128::zero(), Uint128::zero(), Uint128::zero()],
        &[
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "asset1".to_string(),
                },
                amount: Default::default(),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "asset2".to_string(),
                },
                amount: Default::default(),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "asset3".to_string(),
                },
                amount: Default::default(),
            },
        ],
        Default::default(),
        Default::default(),
    );

    match res {
        Ok(_) => panic!("should return ContractError::Std"),
        Err(ContractError::Std { .. }) => (),
        _ => panic!("should return ContractError::Std"),
    }
}
