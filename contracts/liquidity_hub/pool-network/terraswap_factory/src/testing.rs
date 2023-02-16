use cosmwasm_std::testing::{
    mock_dependencies_with_balance, mock_env, mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR,
};
use cosmwasm_std::{
    attr, coin, from_binary, to_binary, Api, CanonicalAddr, CosmosMsg, Decimal, OwnedDeps, Reply,
    ReplyOn, Response, SubMsg, SubMsgResponse, SubMsgResult, Uint128, WasmMsg,
};

use pool_network::asset::{AssetInfo, PairInfo, PairInfoRaw, PairType};
use pool_network::factory::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, NativeTokenDecimalsResponse, QueryMsg,
};
use pool_network::mock_querier::{mock_dependencies, WasmMockQuerier};
use pool_network::pair::{
    InstantiateMsg as PairInstantiateMsg, MigrateMsg as PairMigrateMsg, PoolFee,
};
use white_whale::fee::Fee;

use crate::contract::{execute, instantiate, migrate, query, reply};
use crate::error::ContractError;
use crate::state::{pair_key, TmpPairInfo, PAIRS, TMP_PAIR_INFO};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
        fee_collector_addr: "collector".to_string(),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(123u64, config_res.token_code_id);
    assert_eq!(321u64, config_res.pair_code_id);
    assert_eq!("addr0000".to_string(), config_res.owner);
}

#[test]
fn can_migrate_contract() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
        fee_collector_addr: "collector".to_string(),
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
fn update_config() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
        fee_collector_addr: "collector".to_string(),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // update owner
    let info = mock_info("addr0000", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: Some("addr0001".to_string()),
        pair_code_id: None,
        token_code_id: None,
        fee_collector_addr: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(123u64, config_res.token_code_id);
    assert_eq!(321u64, config_res.pair_code_id);
    assert_eq!("addr0001".to_string(), config_res.owner);
    assert_eq!("collector".to_string(), config_res.fee_collector_addr);

    // update left items
    let env = mock_env();
    let info = mock_info("addr0001", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        pair_code_id: Some(100u64),
        token_code_id: Some(200u64),
        fee_collector_addr: Some("new_collector".to_string()),
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(200u64, config_res.token_code_id);
    assert_eq!(100u64, config_res.pair_code_id);
    assert_eq!("addr0001".to_string(), config_res.owner);
    assert_eq!("new_collector".to_string(), config_res.fee_collector_addr);

    // Unauthorized err
    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        fee_collector_addr: None,
        pair_code_id: None,
        token_code_id: None,
    };

    let res = execute(deps.as_mut(), env, info, msg);
    match res {
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("Must return ContractError::Unauthorized error"),
    }
}

fn init(
    mut deps: OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let msg = InstantiateMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
        fee_collector_addr: "collector".to_string(),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    deps.querier.with_token_balances(&[(
        &"asset0001".to_string(),
        &[(&"addr0000".to_string(), &Uint128::zero())],
    )]);
    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    deps
}

#[test]
fn create_pair() {
    let mut deps = mock_dependencies(&[coin(10u128, "uusd".to_string())]);
    deps = init(deps);
    deps.querier
        .with_pool_factory(&[], &[("uusd".to_string(), 6u8)]);
    let asset_infos = [
        AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        AssetInfo::Token {
            contract_addr: "asset0001".to_string(),
        },
    ];

    let msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos.clone(),
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
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "create_pair"),
            attr("pair", "uusd-mAAPL"),
            attr("pair_label", "uusd-mAAPL pair"),
            attr("pair_type", "ConstantProduct")
        ]
    );
    assert_eq!(
        res.messages,
        vec![SubMsg {
            id: 1,
            gas_limit: None,
            reply_on: ReplyOn::Success,
            msg: WasmMsg::Instantiate {
                msg: to_binary(&PairInstantiateMsg {
                    asset_infos: asset_infos.clone(),
                    token_code_id: 123u64,
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
                    pair_type: PairType::ConstantProduct
                })
                .unwrap(),
                code_id: 321u64,
                funds: vec![],
                label: "uusd-mAAPL pair".to_string(),
                admin: Some(MOCK_CONTRACT_ADDR.to_string()),
            }
            .into(),
        },]
    );

    let raw_infos = [
        asset_infos[0].to_raw(deps.as_ref().api).unwrap(),
        asset_infos[1].to_raw(deps.as_ref().api).unwrap(),
    ];

    assert_eq!(
        TMP_PAIR_INFO.load(&deps.storage).unwrap(),
        TmpPairInfo {
            asset_infos: raw_infos.clone(),
            pair_key: pair_key(&raw_infos),
            asset_decimals: [6u8, 8u8],
            pair_type: PairType::ConstantProduct
        }
    );
}

#[test]
fn create_stableswap_pair() {
    let mut deps = mock_dependencies(&[coin(10u128, "uusd".to_string())]);
    deps = init(deps);
    deps.querier
        .with_pool_factory(&[], &[("uusd".to_string(), 6u8)]);
    let asset_infos = [
        AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        AssetInfo::Token {
            contract_addr: "asset0001".to_string(),
        },
    ];

    let msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos.clone(),
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
        pair_type: PairType::StableSwap { amp: 100 },
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "create_pair"),
            attr("pair", "uusd-mAAPL"),
            attr("pair_label", "uusd-mAAPL pair"),
            attr("pair_type", "StableSwap")
        ]
    );
    assert_eq!(
        res.messages,
        vec![SubMsg {
            id: 1,
            gas_limit: None,
            reply_on: ReplyOn::Success,
            msg: WasmMsg::Instantiate {
                msg: to_binary(&PairInstantiateMsg {
                    asset_infos: asset_infos.clone(),
                    token_code_id: 123u64,
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
                    pair_type: PairType::StableSwap { amp: 100 }
                })
                .unwrap(),
                code_id: 321u64,
                funds: vec![],
                label: "uusd-mAAPL pair".to_string(),
                admin: Some(MOCK_CONTRACT_ADDR.to_string()),
            }
            .into(),
        },]
    );

    let raw_infos = [
        asset_infos[0].to_raw(deps.as_ref().api).unwrap(),
        asset_infos[1].to_raw(deps.as_ref().api).unwrap(),
    ];

    assert_eq!(
        TMP_PAIR_INFO.load(&deps.storage).unwrap(),
        TmpPairInfo {
            asset_infos: raw_infos.clone(),
            pair_key: pair_key(&raw_infos),
            asset_decimals: [6u8, 8u8],
            pair_type: PairType::StableSwap { amp: 100 }
        }
    );
}

#[test]
fn create_pair_native_token_and_ibc_token() {
    let mut deps = mock_dependencies(&[
        coin(10u128, "uusd".to_string()),
        coin(
            10u128,
            "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2".to_string(),
        ),
    ]);
    deps = init(deps);
    deps.querier.with_pool_factory(
        &[],
        &[
            ("uusd".to_string(), 6u8),
            (
                "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2".to_string(),
                6u8,
            ),
        ],
    );

    let asset_infos = [
        AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        AssetInfo::NativeToken {
            denom: "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2"
                .to_string(),
        },
    ];

    let msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos.clone(),
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
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "create_pair"),
            attr("pair", "uusd-ibc/2739...5EB2"),
            attr("pair_label", "uusd-ibc/2739...5EB2 pair"),
            attr("pair_type", "ConstantProduct")
        ]
    );
    assert_eq!(
        res.messages,
        vec![SubMsg {
            id: 1,
            gas_limit: None,
            reply_on: ReplyOn::Success,
            msg: WasmMsg::Instantiate {
                msg: to_binary(&PairInstantiateMsg {
                    asset_infos: asset_infos.clone(),
                    token_code_id: 123u64,
                    asset_decimals: [6u8, 6u8],
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
                })
                .unwrap(),
                code_id: 321u64,
                funds: vec![],
                label: "uusd-ibc/2739...5EB2 pair".to_string(),
                admin: Some(MOCK_CONTRACT_ADDR.to_string()),
            }
            .into(),
        },]
    );

    let raw_infos = [
        asset_infos[0].to_raw(deps.as_ref().api).unwrap(),
        asset_infos[1].to_raw(deps.as_ref().api).unwrap(),
    ];

    assert_eq!(
        TMP_PAIR_INFO.load(&deps.storage).unwrap(),
        TmpPairInfo {
            asset_infos: raw_infos.clone(),
            pair_key: pair_key(&raw_infos),
            asset_decimals: [6u8, 6u8],
            pair_type: PairType::ConstantProduct,
        }
    );
}

#[test]
fn create_ibc_tokens_pair() {
    let mut deps = mock_dependencies(&[
        coin(
            10u128,
            "ibc/4CD525F166D32B0132C095F353F4C6F033B0FF5C49141470D1EFDA1D63303D04".to_string(),
        ),
        coin(
            10u128,
            "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2".to_string(),
        ),
    ]);
    deps = init(deps);
    deps.querier.with_pool_factory(
        &[],
        &[
            (
                "ibc/4CD525F166D32B0132C095F353F4C6F033B0FF5C49141470D1EFDA1D63303D04".to_string(),
                6u8,
            ),
            (
                "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2".to_string(),
                6u8,
            ),
        ],
    );

    let asset_infos = [
        AssetInfo::NativeToken {
            denom: "ibc/4CD525F166D32B0132C095F353F4C6F033B0FF5C49141470D1EFDA1D63303D04"
                .to_string(),
        },
        AssetInfo::NativeToken {
            denom: "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2"
                .to_string(),
        },
    ];

    let msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos.clone(),
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
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "create_pair"),
            attr("pair", "ibc/4CD5...3D04-ibc/2739...5EB2"),
            attr("pair_label", "ibc/4CD5...3D04-ibc/2739...5EB2 pair"),
            attr("pair_type", "ConstantProduct")
        ]
    );
    assert_eq!(
        res.messages,
        vec![SubMsg {
            id: 1,
            gas_limit: None,
            reply_on: ReplyOn::Success,
            msg: WasmMsg::Instantiate {
                msg: to_binary(&PairInstantiateMsg {
                    asset_infos: asset_infos.clone(),
                    token_code_id: 123u64,
                    asset_decimals: [6u8, 6u8],
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
                })
                .unwrap(),
                code_id: 321u64,
                funds: vec![],
                label: "ibc/4CD5...3D04-ibc/2739...5EB2 pair".to_string(),
                admin: Some(MOCK_CONTRACT_ADDR.to_string()),
            }
            .into(),
        },]
    );

    let raw_infos = [
        asset_infos[0].to_raw(deps.as_ref().api).unwrap(),
        asset_infos[1].to_raw(deps.as_ref().api).unwrap(),
    ];

    assert_eq!(
        TMP_PAIR_INFO.load(&deps.storage).unwrap(),
        TmpPairInfo {
            asset_infos: raw_infos.clone(),
            pair_key: pair_key(&raw_infos),
            asset_decimals: [6u8, 6u8],
            pair_type: PairType::ConstantProduct,
        }
    );
}

#[cfg(feature = "injective")]
#[test]
fn create_pair_ethereum_asset_and_ibc_token() {
    let mut deps = mock_dependencies(&[
        coin(
            10u128,
            "peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5".to_string(),
        ),
        coin(
            10u128,
            "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2".to_string(),
        ),
    ]);
    deps = init(deps);
    deps.querier.with_pool_factory(
        &[],
        &[
            (
                "peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5".to_string(),
                6u8,
            ),
            (
                "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2".to_string(),
                6u8,
            ),
        ],
    );

    let asset_infos = [
        AssetInfo::NativeToken {
            denom: "peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5".to_string(),
        },
        AssetInfo::NativeToken {
            denom: "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2"
                .to_string(),
        },
    ];

    let msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos.clone(),
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
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "create_pair"),
            attr("pair", "peggy0x87a...1B5-ibc/2739...5EB2"),
            attr("pair_label", "peggy0x87a...1B5-ibc/2739...5EB2 pair"),
            attr("pair_type", "ConstantProduct")
        ]
    );
    assert_eq!(
        res.messages,
        vec![SubMsg {
            id: 1,
            gas_limit: None,
            reply_on: ReplyOn::Success,
            msg: WasmMsg::Instantiate {
                msg: to_binary(&PairInstantiateMsg {
                    asset_infos: asset_infos.clone(),
                    token_code_id: 123u64,
                    asset_decimals: [6u8, 6u8],
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
                    pair_type: PairType::ConstantProduct
                })
                .unwrap(),
                code_id: 321u64,
                funds: vec![],
                label: "peggy0x87a...1B5-ibc/2739...5EB2 pair".to_string(),
                admin: Some(MOCK_CONTRACT_ADDR.to_string()),
            }
            .into(),
        },]
    );

    let raw_infos = [
        asset_infos[0].to_raw(deps.as_ref().api).unwrap(),
        asset_infos[1].to_raw(deps.as_ref().api).unwrap(),
    ];

    assert_eq!(
        TMP_PAIR_INFO.load(&deps.storage).unwrap(),
        TmpPairInfo {
            asset_infos: raw_infos.clone(),
            pair_key: pair_key(&raw_infos),
            asset_decimals: [6u8, 6u8],
            pair_type: PairType::ConstantProduct
        }
    );
}

#[test]
fn fail_to_create_same_pair() {
    let mut deps = mock_dependencies(&[coin(10u128, "uusd".to_string())]);
    deps = init(deps);

    let asset_infos = [
        AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
    ];

    let msg = ExecuteMsg::CreatePair {
        asset_infos,
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
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    let res = execute(deps.as_mut(), env, info, msg);
    match res {
        Ok(_) => panic!("Should return ContractError::SameAsset"),
        Err(ContractError::SameAsset { .. }) => (),
        _ => panic!("Should return ContractError::SameAsset"),
    }
}

#[test]
fn fail_to_create_existing_pair() {
    let mut deps = mock_dependencies(&[coin(10u128, "uusd".to_string())]);
    deps = init(deps);
    deps.querier
        .with_pool_factory(&[], &[("uusd".to_string(), 6u8)]);
    let asset_infos = [
        AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        AssetInfo::Token {
            contract_addr: "asset0001".to_string(),
        },
    ];

    let msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos.clone(),
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
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    // inject pair into PAIRS
    let raw_infos = [
        asset_infos[0].to_raw(deps.as_ref().api).unwrap(),
        asset_infos[1].to_raw(deps.as_ref().api).unwrap(),
    ];
    let pair_key = pair_key(&raw_infos);

    PAIRS
        .save(
            &mut deps.storage,
            &pair_key,
            &PairInfoRaw {
                liquidity_token: deps.api.addr_canonicalize("lp_token").unwrap(),
                contract_addr: deps.api.addr_canonicalize("pair_contract").unwrap(),
                asset_infos: raw_infos,
                asset_decimals: [6u8, 6u8],
                pair_type: PairType::ConstantProduct,
            },
        )
        .unwrap();

    // try to recreate the same pair
    let res = execute(deps.as_mut(), env, info, msg);
    match res {
        Ok(_) => panic!("Should return ContractError::ExistingPair"),
        Err(ContractError::ExistingPair { .. }) => (),
        _ => panic!("Should return ContractError::ExistingPair"),
    }
}

#[test]
fn fail_to_create_pair_with_inactive_denoms() {
    let mut deps = mock_dependencies(&[coin(10u128, "uusd".to_string())]);
    deps = init(deps);

    let asset_infos = [
        AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        AssetInfo::NativeToken {
            denom: "uxxx".to_string(),
        },
    ];

    let msg = ExecuteMsg::CreatePair {
        asset_infos,
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
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    let res = execute(deps.as_mut(), env, info, msg);
    match res {
        Ok(_) => panic!("Should return ContractError::InvalidAsset"),
        Err(ContractError::InvalidAsset { .. }) => (),
        _ => panic!("Should return ContractError::InvalidAsset"),
    }
}

#[test]
fn fail_to_create_pair_with_invalid_denom() {
    let mut deps = mock_dependencies(&[coin(10u128, "valid".to_string())]);
    deps = init(deps);
    deps.querier
        .with_pool_factory(&[], &[("valid".to_string(), 6u8)]);

    let asset_infos = [
        AssetInfo::NativeToken {
            denom: "valid".to_string(),
        },
        AssetInfo::NativeToken {
            denom: "invalid".to_string(),
        },
    ];

    let msg = ExecuteMsg::CreatePair {
        asset_infos,
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
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    let res = execute(deps.as_mut(), env, info, msg);
    match res {
        Ok(_) => panic!("Should return ContractError::InvalidAsset"),
        Err(ContractError::InvalidAsset { asset }) => assert_eq!("invalid", asset),
        _ => panic!("Should return ContractError::InvalidAsset"),
    }

    let asset_infos = [
        AssetInfo::NativeToken {
            denom: "invalid".to_string(),
        },
        AssetInfo::NativeToken {
            denom: "valid".to_string(),
        },
    ];

    let msg = ExecuteMsg::CreatePair {
        asset_infos,
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
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    let res = execute(deps.as_mut(), env, info, msg);
    match res {
        Ok(_) => panic!("Should return ContractError::InvalidAsset"),
        Err(ContractError::InvalidAsset { asset }) => assert_eq!("invalid", asset),
        _ => panic!("Should return ContractError::InvalidAsset"),
    }
}

#[test]
fn fail_to_create_pair_with_unknown_token() {
    let mut deps = mock_dependencies(&[coin(10u128, "uusd".to_string())]);

    let msg = InstantiateMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
        fee_collector_addr: "collector".to_string(),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    let asset_infos = [
        AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        },
        AssetInfo::Token {
            contract_addr: "xxx".to_string(),
        },
    ];

    let msg = ExecuteMsg::CreatePair {
        asset_infos,
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
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    let res = execute(deps.as_mut(), env, info, msg);
    match res {
        Ok(_) => panic!("Should return ContractError::InvalidAsset"),
        Err(ContractError::InvalidAsset { .. }) => (),
        _ => panic!("Should return ContractError::InvalidAsset"),
    }
}

#[test]
fn fail_to_create_pair_with_unknown_ibc_token() {
    let mut deps = mock_dependencies_with_balance(&[coin(10u128, "uusd".to_string())]);

    let msg = InstantiateMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
        fee_collector_addr: "collector".to_string(),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    let asset_infos = [
        AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        },
        AssetInfo::NativeToken {
            denom: "ibc/HA".to_string(),
        },
    ];

    let msg = ExecuteMsg::CreatePair {
        asset_infos,
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
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    let res = execute(deps.as_mut(), env, info, msg);
    match res {
        Ok(_) => panic!("Should return ContractError::InvalidAsset"),
        Err(ContractError::InvalidAsset { .. }) => (),
        _ => panic!("Should return ContractError::InvalidAsset"),
    }
}

#[test]
fn reply_test() {
    let mut deps = mock_dependencies(&[]);

    deps.querier.with_token_balances(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        &[
            (&"asset0000".to_string(), &Uint128::from(100u128)),
            (&"asset0001".to_string(), &Uint128::from(100u128)),
        ],
    )]);

    let asset_infos = [
        AssetInfo::Token {
            contract_addr: "asset0000".to_string(),
        },
        AssetInfo::Token {
            contract_addr: "asset0001".to_string(),
        },
    ];

    let raw_infos = [
        asset_infos[0].to_raw(deps.as_ref().api).unwrap(),
        asset_infos[1].to_raw(deps.as_ref().api).unwrap(),
    ];

    let pair_key = pair_key(&raw_infos);
    TMP_PAIR_INFO
        .save(
            &mut deps.storage,
            &TmpPairInfo {
                asset_infos: raw_infos,
                pair_key,
                asset_decimals: [8u8, 8u8],
                pair_type: PairType::ConstantProduct,
            },
        )
        .unwrap();

    let reply_msg = Reply {
        id: 1,
        result: SubMsgResult::Ok(SubMsgResponse {
            events: vec![],
            data: Some(vec![10, 4, 48, 48, 48, 48].into()),
        }),
    };

    // register pool pair querier
    deps.querier.with_pool_factory(
        &[(
            &"0000".to_string(),
            &PairInfo {
                asset_infos: [
                    AssetInfo::Token {
                        contract_addr: "asset0000".to_string(),
                    },
                    AssetInfo::Token {
                        contract_addr: "asset0001".to_string(),
                    },
                ],
                contract_addr: "0000".to_string(),
                liquidity_token: "liquidity0000".to_string(),
                asset_decimals: [8u8, 8u8],
                pair_type: PairType::ConstantProduct,
            },
        )],
        &[],
    );

    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Pair {
            asset_infos: asset_infos.clone(),
        },
    )
    .unwrap();

    let pair_res: PairInfo = from_binary(&query_res).unwrap();
    assert_eq!(
        pair_res,
        PairInfo {
            liquidity_token: "liquidity0000".to_string(),
            contract_addr: "0000".to_string(),
            asset_infos,
            asset_decimals: [8u8, 8u8],
            pair_type: PairType::ConstantProduct,
        }
    );
}

#[test]
fn normal_add_allow_native_token() {
    let mut deps = mock_dependencies(&[coin(1u128, "uluna".to_string())]);
    deps = init(deps);

    let msg = ExecuteMsg::AddNativeTokenDecimals {
        denom: "uluna".to_string(),
        decimals: 6u8,
    };

    let info = mock_info("addr0000", &[]);

    assert_eq!(
        execute(deps.as_mut(), mock_env(), info, msg).unwrap(),
        Response::new().add_attributes(vec![
            ("action", "add_allow_native_token"),
            ("denom", "uluna"),
            ("decimals", "6"),
        ])
    );

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::NativeTokenDecimals {
            denom: "uluna".to_string(),
        },
    )
    .unwrap();
    let res: NativeTokenDecimalsResponse = from_binary(&res).unwrap();
    assert_eq!(6u8, res.decimals)
}

#[test]
fn failed_add_allow_native_token_with_non_admin() {
    let mut deps = mock_dependencies(&[coin(1u128, "uluna".to_string())]);
    deps = init(deps);

    let msg = ExecuteMsg::AddNativeTokenDecimals {
        denom: "uluna".to_string(),
        decimals: 6u8,
    };

    let info = mock_info("noadmin", &[]);

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Ok(_) => panic!("should return ContractError::Unauthorized"),
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("should return ContractError::Unauthorized"),
    }
}

#[test]
fn failed_add_allow_native_token_with_zero_factory_balance() {
    let mut deps = mock_dependencies(&[coin(0u128, "uluna".to_string())]);
    deps = init(deps);

    let msg = ExecuteMsg::AddNativeTokenDecimals {
        denom: "uluna".to_string(),
        decimals: 6u8,
    };

    let info = mock_info("addr0000", &[]);

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Ok(_) => panic!("should return ContractError::InvalidVerificationBalance"),
        Err(ContractError::InvalidVerificationBalance {}) => (),
        _ => panic!("should return ContractError::InvalidVerificationBalance"),
    }
}

#[test]
fn append_add_allow_native_token_with_already_exist_token() {
    let mut deps = mock_dependencies(&[coin(1u128, "uluna".to_string())]);
    deps = init(deps);

    let msg = ExecuteMsg::AddNativeTokenDecimals {
        denom: "uluna".to_string(),

        decimals: 6u8,
    };

    let info = mock_info("addr0000", &[]);

    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::NativeTokenDecimals {
            denom: "uluna".to_string(),
        },
    )
    .unwrap();
    let res: NativeTokenDecimalsResponse = from_binary(&res).unwrap();
    assert_eq!(6u8, res.decimals);

    let msg = ExecuteMsg::AddNativeTokenDecimals {
        denom: "uluna".to_string(),
        decimals: 7u8,
    };

    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::NativeTokenDecimals {
            denom: "uluna".to_string(),
        },
    )
    .unwrap();
    let res: NativeTokenDecimalsResponse = from_binary(&res).unwrap();
    assert_eq!(7u8, res.decimals)
}

#[test]
fn execute_transactions_unauthorized() {
    let mut deps = mock_dependencies(&[coin(10u128, "uusd".to_string())]);
    deps = init(deps);
    deps.querier
        .with_pool_factory(&[], &[("uusd".to_string(), 6u8)]);
    let asset_infos = [
        AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        AssetInfo::Token {
            contract_addr: "asset0001".to_string(),
        },
    ];
    let env = mock_env();
    // unauthorized user
    let info = mock_info("unauthorized", &[]);

    // Try executing ExecuteMsg::CreatePair
    let msg = ExecuteMsg::CreatePair {
        asset_infos,
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
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);

    match res {
        Ok(_) => panic!("Must return ContractError::Unauthorized"),
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("Must return ContractError::Unauthorized"),
    }

    // Try executing ExecuteMsg::AddNativeTokenDecimals
    let msg = ExecuteMsg::AddNativeTokenDecimals {
        denom: "any".to_string(),
        decimals: 6,
    };

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);

    match res {
        Ok(_) => panic!("Must return ContractError::Unauthorized"),
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("Must return ContractError::Unauthorized"),
    }

    // Try executing ExecuteMsg::UpdateConfig
    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        fee_collector_addr: None,
        token_code_id: None,
        pair_code_id: None,
    };

    let res = execute(deps.as_mut(), env, info, msg);

    match res {
        Ok(_) => panic!("Must return ContractError::Unauthorized"),
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("Must return ContractError::Unauthorized"),
    }
}

#[test]
fn normal_migrate_pair() {
    let mut deps = mock_dependencies(&[coin(1u128, "uluna".to_string())]);
    deps = init(deps);

    let msg = ExecuteMsg::MigratePair {
        code_id: Some(123u64),
        contract: "contract0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);

    assert_eq!(
        execute(deps.as_mut(), mock_env(), info, msg).unwrap(),
        Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Migrate {
            contract_addr: "contract0000".to_string(),
            new_code_id: 123u64,
            msg: to_binary(&PairMigrateMsg {}).unwrap(),
        })),
    );
}

#[test]
fn normal_migrate_pair_with_none_code_id_will_config_code_id() {
    let mut deps = mock_dependencies(&[coin(1u128, "uluna".to_string())]);
    deps = init(deps);

    let msg = ExecuteMsg::MigratePair {
        code_id: None,
        contract: "contract0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);

    assert_eq!(
        execute(deps.as_mut(), mock_env(), info, msg).unwrap(),
        Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Migrate {
            contract_addr: "contract0000".to_string(),
            new_code_id: 321u64,
            msg: to_binary(&PairMigrateMsg {}).unwrap(),
        })),
    );
}

#[test]
fn failed_migrate_pair_with_no_admin() {
    let mut deps = mock_dependencies(&[coin(1u128, "uluna".to_string())]);
    deps = init(deps);

    let msg = ExecuteMsg::MigratePair {
        code_id: None,
        contract: "contract0000".to_string(),
    };

    let info = mock_info("noadmin", &[]);

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Ok(_) => panic!("should return ContractError::Unauthorized"),
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("should return ContractError::Unauthorized"),
    }
}

#[test]
fn delete_pair() {
    let mut deps = mock_dependencies(&[coin(10u128, "uusd".to_string())]);
    deps = init(deps);
    deps.querier
        .with_pool_factory(&[], &[("uusd".to_string(), 6u8)]);
    let asset_infos = [
        AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        AssetInfo::Token {
            contract_addr: "asset0001".to_string(),
        },
    ];

    let raw_infos = [
        asset_infos[0].to_raw(&deps.api).unwrap(),
        asset_infos[1].to_raw(&deps.api).unwrap(),
    ];

    let pair_key_vec = pair_key(&raw_infos);

    PAIRS
        .save(
            &mut deps.storage,
            &pair_key_vec,
            &PairInfoRaw {
                liquidity_token: CanonicalAddr(cosmwasm_std::Binary(vec![])),
                contract_addr: deps.api.addr_canonicalize("pair0000").unwrap(),
                asset_infos: raw_infos,
                asset_decimals: [6, 6],
                pair_type: PairType::ConstantProduct,
            },
        )
        .unwrap();

    let pair = PAIRS.load(&deps.storage, &pair_key_vec);

    assert!(pair.is_ok(), "pair key should exist");

    let msg = ExecuteMsg::RemovePair { asset_infos };
    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "remove_pair"),
            attr("pair_contract_addr", "pair0000",),
        ]
    );

    let pair = PAIRS.load(&deps.storage, &pair_key_vec);

    assert!(pair.is_err(), "pair key should not exist");
}

#[test]
fn delete_pair_failed_if_not_found() {
    let mut deps = mock_dependencies(&[coin(10u128, "uusd".to_string())]);
    deps = init(deps);
    deps.querier
        .with_pool_factory(&[], &[("uusd".to_string(), 6u8)]);
    let asset_infos = [
        AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        AssetInfo::Token {
            contract_addr: "asset0001".to_string(),
        },
    ];

    let msg = ExecuteMsg::RemovePair { asset_infos };
    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), env, info, msg);

    match res {
        Ok(_) => panic!("should return ContractError::UnExistingPair"),
        Err(ContractError::UnExistingPair {}) => (),
        _ => panic!("should return ContractError::UnExistingPair"),
    }
}

#[test]
fn update_pair_config() {
    let mut deps = mock_dependencies(&[coin(10u128, "uusd".to_string())]);
    deps = init(deps);

    let msg = ExecuteMsg::UpdatePairConfig {
        pair_addr: "pair_addr".to_string(),
        owner: Some("new_owner".to_string()),
        fee_collector_addr: None,
        pool_fees: Some(PoolFee {
            protocol_fee: Fee {
                share: Decimal::percent(3u64),
            },
            swap_fee: Fee {
                share: Decimal::percent(5u64),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
        }),
        feature_toggle: None,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(
        res,
        Response::new()
            .add_attributes(vec![attr("action", "update_pair_config"),])
            .add_message(WasmMsg::Execute {
                contract_addr: "pair_addr".to_string(),
                funds: vec![],
                msg: to_binary(&pool_network::pair::ExecuteMsg::UpdateConfig {
                    owner: Some("new_owner".to_string()),
                    fee_collector_addr: None,
                    pool_fees: Some(PoolFee {
                        protocol_fee: Fee {
                            share: Decimal::percent(3u64),
                        },
                        swap_fee: Fee {
                            share: Decimal::percent(5u64),
                        },
                        burn_fee: Fee {
                            share: Decimal::zero(),
                        },
                    }),
                    feature_toggle: None,
                })
                .unwrap()
            })
    );
}
