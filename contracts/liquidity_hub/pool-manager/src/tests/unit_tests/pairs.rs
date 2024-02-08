use cosmwasm_std::testing::{
    mock_dependencies_with_balance, mock_env, mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR,
};
use cosmwasm_std::{
    attr, coin, from_binary, to_binary, Api, CanonicalAddr, Coin, CosmosMsg, Decimal, OwnedDeps,
    Reply, ReplyOn, Response, SubMsg, SubMsgResponse, SubMsgResult, Uint128, WasmMsg,
};

use white_whale_std::fee::Fee;
use white_whale_std::pool_network;
use white_whale_std::pool_network::asset::{AssetInfo, AssetInfoRaw, PairInfo, PairInfoRaw, PairType};
use white_whale_std::pool_network::factory::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, NativeTokenDecimalsResponse, QueryMsg,
};
use white_whale_std::pool_network::mock_querier::{
    mock_dependencies, mock_dependencies_trio, WasmMockQuerier, WasmMockTrioQuerier,
};
use white_whale_std::pool_network::pair::{
    InstantiateMsg as PairInstantiateMsg, MigrateMsg as PairMigrateMsg, PoolFee,
};
use white_whale_std::pool_network::trio::{
    InstantiateMsg as TrioInstantiateMsg, MigrateMsg as TrioMigrateMsg, PoolFee as TrioPoolFee,
};

use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use white_whale_std::pool_manager::InstantiateMsg as SingleSwapInstantiateMsg;
use crate::state::{pair_key, PAIRS};
use test_case::test_case;
#[cfg(test)]
mod pair_creation_tests {
    use super::*;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{coin, coins, Binary, Decimal, DepsMut, Uint128};
    use cw20::MinterResponse;
    use white_whale_std::pool_network::asset::Asset;
    use crate::tests::mock_querier::mock_dependencies;

    // use crate::msg::{AssetInfo, ExecuteMsg, Fee, PairType, PoolFee};
    use white_whale_std::pool_manager::ExecuteMsg;
    use white_whale_std::pool_network::pair;
    use crate::state::{add_allow_native_token};
    use crate::token::InstantiateMsg as TokenInstantiateMsg;
    use cosmwasm_std::attr;
    use cosmwasm_std::SubMsg;
    use cosmwasm_std::WasmMsg;
    use test_case::test_case;

    // Constants for testing
    const MOCK_CONTRACT_ADDR: &str = "contract_addr";

    #[test]
    fn create_pair() {
        let mut deps = mock_dependencies(&[coin(10u128, "uusd".to_string())]);
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
        deps.querier
            .with_pool_factory(&[], &[("uusd".to_string(), 6u8)]);
        deps.querier.with_token_balances(&[(
            &"asset0001".to_string(),
            &[(&"addr0000".to_string(), &Uint128::new(1000000u128))],
        )]);
        let asset_infos = [
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            AssetInfo::Token {
                contract_addr: "asset0001".to_string(),
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
            pair_identifier: Some("uusd-mAAPL".to_string()),
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
                attr("pair", "uusd-mAAPL"),
                attr("pair_label", "uusd-mAAPL"),
                attr("pair_type", "ConstantProduct"),
            ]
        );
        let seed = format!(
            "{}{}{}",
            "uusd-mAAPL".to_string(),
            info.sender.into_string(),
            env.block.height
        );
        let salt = Binary::from(seed.as_bytes());

        assert_eq!(
            res.messages,
            vec![SubMsg {
                id: 0,
                gas_limit: None,
                reply_on: ReplyOn::Never,
                msg: WasmMsg::Instantiate2 {
                    msg: to_binary(&TokenInstantiateMsg {
                        name: "uusd-mAAPL-LP".to_string(),
                        symbol: "uLP".to_string(),
                        decimals: 6,
                        initial_balances: vec![],
                        mint: Some(MinterResponse {
                            minter: env.contract.address.to_string(),
                            cap: None,
                        }),
                    })
                    .unwrap(),
                    code_id: 11u64,
                    funds: vec![],
                    label: "uusd-mAAPL-LP".to_string(),
                    admin: None,
                    salt
                }
                .into(),
            },]
        );
    }

    #[test]
    fn create_stableswap_pair() {
        let mut deps = mock_dependencies(&[coin(10u128, "uusd".to_string())]);
        // deps.api = Box::new(MockApi::default());
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

        deps.querier
            .with_pool_factory(&[], &[("uusd".to_string(), 6u8)]);
        deps.querier.with_token_balances(&[(
            &"asset0001".to_string(),
            &[(&"addr0000".to_string(), &Uint128::new(1000000u128))],
        )]);

        let asset_infos = [
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            AssetInfo::Token {
                contract_addr: "asset0001".to_string(),
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
            pair_type: PairType::StableSwap { amp: 100 },
            token_factory_lp: false,
            pair_identifier: Some("uusd-mAAPL".to_string()),
        };

        let env = mock_env();
        let info = mock_info("addr0000", &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(1000000u128),
        }]);
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        let seed = format!(
            "{}{}{}",
            "uusd-mAAPL".to_string(),
            info.sender.into_string(),
            env.block.height
        );
        let salt = Binary::from(seed.as_bytes());
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "create_pair"),
                attr("pair", "uusd-mAAPL"),
                attr("pair_label", "uusd-mAAPL"),
                attr("pair_type", "StableSwap"),
            ]
        );

        assert_eq!(
            res.messages,
            vec![SubMsg {
                id: 0,
                gas_limit: None,
                reply_on: ReplyOn::Never,
                msg: WasmMsg::Instantiate2 {
                    msg: to_binary(&TokenInstantiateMsg {
                        name: "uusd-mAAPL-LP".to_string(),
                        symbol: "uLP".to_string(),
                        decimals: 6,
                        initial_balances: vec![],
                        mint: Some(MinterResponse {
                            minter: env.contract.address.to_string(),
                            cap: None,
                        }),
                    })
                    .unwrap(),
                    code_id: 11u64,
                    funds: vec![],
                    label: "uusd-mAAPL-LP".to_string(),
                    admin: None,
                    salt
                }
                .into(),
            },]
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

        deps.querier.with_pool_factory(
            &[],
            &[
                ("uusd".to_string(), 6u8),
                (
                    "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2"
                        .to_string(),
                    6u8,
                ),
            ],
        );
        deps.querier.with_token_balances(&[(
            &"asset0001".to_string(),
            &[(&"addr0000".to_string(), &Uint128::new(1000000u128))],
        )]);

        // deps = init(deps);
        // deps.querier.with_pool_factory(
        //     &[],
        //     &[
        //         ("uusd".to_string(), 6u8),
        //         (
        //             "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2".to_string(),
        //             6u8,
        //         ),
        //     ],
        // );

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
        let info = mock_info("addr0000", &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(1000000u128),
        }]);
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "create_pair"),
                attr("pair", "uusd-ibc/2739...5EB2"),
                attr("pair_label", "uusd-ibc/2739...5EB2"),
                attr("pair_type", "ConstantProduct"),
            ]
        );
        let seed = format!(
            "{}{}{}",
            "uusd-ibc/2739...5EB2".to_string(),
            info.sender.into_string(),
            env.block.height
        );
        let salt = Binary::from(seed.as_bytes());
        assert_eq!(
            res.messages,
            vec![SubMsg {
                id: 0,
                gas_limit: None,
                reply_on: ReplyOn::Never,
                msg: WasmMsg::Instantiate2 {
                    msg: to_binary(&TokenInstantiateMsg {
                        name: "uusd-ibc/2739...5EB2-LP".to_string(),
                        symbol: "uLP".to_string(),
                        decimals: 6,
                        initial_balances: vec![],
                        mint: Some(MinterResponse {
                            minter: env.contract.address.to_string(),
                            cap: None,
                        }),
                    })
                    .unwrap(),
                    code_id: 11u64,
                    funds: vec![],
                    label: "uusd-ibc/2739...5EB2-LP".to_string(),
                    admin: None,
                    salt
                }
                .into(),
            },]
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
        let pair_creation_fee = Asset {
            amount: Uint128::new(1000000u128),
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            }
        };

        

        // Instantiate contract
        let msg = SingleSwapInstantiateMsg {
            fee_collector_addr: "fee_collector_addr".to_string(),
            owner: "owner".to_string(),
            pair_code_id: 10u64,
            token_code_id: 11u64,
            pool_creation_fee: pair_creation_fee.clone(),
        };
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // deps = init(deps);
        deps.querier.with_pool_factory(
            &[],
            &[
                (
                    "ibc/4CD525F166D32B0132C095F353F4C6F033B0FF5C49141470D1EFDA1D63303D04"
                        .to_string(),
                    6u8,
                ),
                (
                    "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2"
                        .to_string(),
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
        let info = mock_info("addr0000", &[coin(1000000u128, "uusd".to_string())]);
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "create_pair"),
                attr("pair", "ibc/4CD5...3D04-ibc/2739...5EB2"),
                attr("pair_label", "ibc/4CD5...3D04-ibc/2739...5EB2"),
                attr("pair_type", "ConstantProduct"),
            ]
        );
        let seed = format!(
            "{}{}{}",
            "ibc/4CD5...3D04-ibc/2739...5EB2".to_string(),
            info.sender.into_string(),
            env.block.height
        );
        let salt = Binary::from(seed.as_bytes());
        assert_eq!(
            res.messages,
            vec![SubMsg {
                id: 0,
                gas_limit: None,
                reply_on: ReplyOn::Never,
                msg: WasmMsg::Instantiate2 {
                    msg: to_binary(&TokenInstantiateMsg {
                        name: "ibc/4CD5...3D04-ibc/2739...5EB2-LP".to_string(),
                        symbol: "uLP".to_string(),
                        decimals: 6,
                        initial_balances: vec![],
                        mint: Some(MinterResponse {
                            minter: env.contract.address.to_string(),
                            cap: None,
                        }),
                    })
                    .unwrap(),
                    code_id: 11u64,
                    funds: vec![],
                    label: "ibc/4CD5...3D04-ibc/2739...5EB2-LP".to_string(),
                    admin: None,
                    salt
                }
                .into(),
            },]
        );
    }

    #[test_case(
        AssetInfo::NativeToken { denom: "uusd".to_string() },
        AssetInfo::NativeToken { denom: "uusd".to_string() },
        ContractError::SameAsset {  } ;
        "fail_to_create_same_pair"
    )]
    #[test_case(
        AssetInfo::NativeToken { denom: "uusd".to_string() },
        AssetInfo::Token { contract_addr: "asset0001".to_string() },
        ContractError::ExistingPair {  } ;
        "fail_to_create_existing_pair"
    )]
    #[test_case(
        AssetInfo::NativeToken { denom: "uusd".to_string() },
        AssetInfo::NativeToken { denom: "uxxx".to_string() },
        ContractError::Std(cosmwasm_std::StdError::generic_err("Querier system error: Cannot parse request: No decimal info exist in: {\"native_token_decimals\":{\"denom\":\"uxxx\"}}".to_string())) ;
                "fail_to_create_pair_with_inactive_denoms"
    )]
    fn test_failures(asset1: AssetInfo, asset2: AssetInfo, expected_error: ContractError) {
        let mut deps = mock_dependencies(&[coin(10000000u128, "uusd".to_string())]);
        // deps = init(deps);
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

        deps.querier
            .with_pool_factory(&[], &[("uusd".to_string(), 6u8)]);
        deps.querier.with_token_balances(&[(
            &"asset0001".to_string(),
            &[(&"addr0000".to_string(), &Uint128::new(1000000u128))],
        )]);
        let msg = white_whale_std::pool_manager::ExecuteMsg::CreatePair {
            asset_infos: [asset1, asset2].to_vec(),
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
        let info = mock_info("addr0000", &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(1000000u128),
        }]);

        if let ContractError::ExistingPair { .. } = expected_error {
            // Create the pair so when we try again below we get ExistingPair provided the error checking is behaving properly
            let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        }

        let res = execute(deps.as_mut(), env, info, msg);
        match res {
            Ok(_) => panic!("Should return expected error"),
            Err(err) => assert_eq!(err, expected_error),
        }
    }
}
