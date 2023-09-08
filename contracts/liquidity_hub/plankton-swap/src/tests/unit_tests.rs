use cosmwasm_std::testing::{
    mock_dependencies_with_balance, mock_env, mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR,
};
use cosmwasm_std::{
    attr, coin, from_binary, to_binary, Api, CanonicalAddr, Coin, CosmosMsg, Decimal, OwnedDeps,
    Reply, ReplyOn, Response, SubMsg, SubMsgResponse, SubMsgResult, Uint128, WasmMsg,
};

use white_whale::fee::Fee;
use white_whale::pool_network;
use white_whale::pool_network::asset::{AssetInfo, AssetInfoRaw, PairInfo, PairInfoRaw, PairType};
use white_whale::pool_network::factory::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, NativeTokenDecimalsResponse, QueryMsg,
};
use white_whale::pool_network::mock_querier::{
    mock_dependencies, mock_dependencies_trio, WasmMockQuerier, WasmMockTrioQuerier,
};
use white_whale::pool_network::pair::{
    InstantiateMsg as PairInstantiateMsg, MigrateMsg as PairMigrateMsg, PoolFee,
};
use white_whale::pool_network::trio::{
    InstantiateMsg as TrioInstantiateMsg, MigrateMsg as TrioMigrateMsg, PoolFee as TrioPoolFee,
};

use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use crate::msg::InstantiateMsg as SingleSwapInstantiateMsg;
use crate::state::{pair_key, PAIRS, TMP_PAIR_INFO};
#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{coin, coins, Decimal, DepsMut, Uint128};
    use white_whale::pool_network::mock_querier::{
        mock_dependencies, mock_dependencies_trio, WasmMockQuerier, WasmMockTrioQuerier,
    };
    use white_whale::pool_network::asset::Asset;
    // use crate::msg::{AssetInfo, ExecuteMsg, Fee, PairType, PoolFee};
    use crate::msg::ExecuteMsg;
    use crate::state::{NAssets, NDecimals, TmpPairInfo, add_allow_native_token};
    use cosmwasm_std::attr;
    use cosmwasm_std::SubMsg;
    use cosmwasm_std::WasmMsg;

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

        let assets: NAssets = NAssets::TWO(asset_infos.clone());

        let msg = ExecuteMsg::CreatePair {
            asset_infos: assets.clone(),
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
        };

        let env = mock_env();
        let info = mock_info(
            "addr0000",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::new(1u128),
            }],
        );
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "create_pair"),
                attr("pair", "uusd-mAAPL"),
                attr("pair_label", "uusd-mAAPL"),
                attr("pair_type", "ConstantProduct"),
            ]
        );
        // assert_eq!(
        //     res.messages,
        //     vec![SubMsg {
        //         id: 1,
        //         gas_limit: None,
        //         reply_on: ReplyOn::Success,
        //         msg: WasmMsg::Instantiate {
        //             msg: to_binary(&PairInstantiateMsg {
        //                 asset_infos: asset_infos.clone(),
        //                 token_code_id: 123u64,
        //                 asset_decimals: [6u8, 8u8],
        //                 pool_fees: PoolFee {
        //                     protocol_fee: Fee {
        //                         share: Decimal::percent(1u64),
        //                     },
        //                     swap_fee: Fee {
        //                         share: Decimal::percent(1u64),
        //                     },
        //                     burn_fee: Fee {
        //                         share: Decimal::zero(),
        //                     },
        //                 },
        //                 fee_collector_addr: "collector".to_string(),
        //                 pair_type: PairType::ConstantProduct,
        //                 token_factory_lp: false,
        //             })
        //             .unwrap(),
        //             code_id: 321u64,
        //             funds: [Coin {
        //                 denom: "uusd".to_string(),
        //                 amount: Uint128::new(1u128),
        //             }]
        //             .to_vec(),
        //             label: "uusd-mAAPL pair".to_string(),
        //             admin: Some(MOCK_CONTRACT_ADDR.to_string()),
        //         }
        //         .into(),
        //     },]
        // );

        // let raw_infos = [
        //     asset_infos[0].to_raw(deps.as_ref().api).unwrap(),
        //     asset_infos[1].to_raw(deps.as_ref().api).unwrap(),
        // ];
    }

    #[test]
    fn create_stableswap_pair() {
        let mut deps = mock_dependencies_with_balance(&[coin(10u128, "uusd".to_string())]);

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

        let asset_infos = [
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            AssetInfo::Token {
                contract_addr: "asset0001".to_string(),
            },
        ];
        let assets: NAssets = NAssets::TWO(asset_infos.clone());

        let msg = ExecuteMsg::CreatePair {
            asset_infos: assets.clone(),
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
                attr("pair_type", "StableSwap"),
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
                        pair_type: PairType::StableSwap { amp: 100 },
                        token_factory_lp: false,
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
                asset_infos: assets,
                pair_key: pair_key(&raw_infos),
                asset_decimals: NDecimals::TWO([6u8, 8u8]),
                pair_type: PairType::StableSwap { amp: 100 },
            }
        );
    }

    #[test]
    fn create_pair_native_token_and_ibc_token() {
        let mut deps = mock_dependencies_with_balance(&[
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
            asset_infos: NAssets::TWO(asset_infos.clone()),
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
                attr("pair_type", "ConstantProduct"),
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
                        token_factory_lp: false,
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
                asset_infos: NAssets::TWO(asset_infos),
                pair_key: pair_key(&raw_infos),
                asset_decimals: NDecimals::TWO([6u8, 6u8]),
                pair_type: PairType::ConstantProduct,
            }
        );
    }

    #[test]
    fn create_ibc_tokens_pair() {
        let mut deps = mock_dependencies_with_balance(&[
            coin(
                10u128,
                "ibc/4CD525F166D32B0132C095F353F4C6F033B0FF5C49141470D1EFDA1D63303D04".to_string(),
            ),
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

        // deps = init(deps);
        // deps.querier.with_pool_factory(
        //     &[],
        //     &[
        //         (
        //             "ibc/4CD525F166D32B0132C095F353F4C6F033B0FF5C49141470D1EFDA1D63303D04".to_string(),
        //             6u8,
        //         ),
        //         (
        //             "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2".to_string(),
        //             6u8,
        //         ),
        //     ],
        // );

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
            asset_infos: NAssets::TWO(asset_infos.clone()),
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
                attr("pair_type", "ConstantProduct"),
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
                        token_factory_lp: false,
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
                asset_infos: NAssets::TWO(asset_infos),
                pair_key: pair_key(&raw_infos),
                asset_decimals: NDecimals::TWO([6u8, 6u8]),
                pair_type: PairType::ConstantProduct,
            }
        );
    }
}
