use crate::contract::{instantiate, query, reply};
use crate::error::ContractError;
use crate::queries::query_pool;
use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{Coin, Decimal, Reply, SubMsgResponse, SubMsgResult, Uint128};
use pool_network::asset::{Asset, AssetInfo, PairType};
use pool_network::mock_querier::mock_dependencies;
use pool_network::pair::{InstantiateMsg, PoolFee, PoolResponse, QueryMsg};
use white_whale::fee::Fee;

#[test]
fn test_simulations_asset_missmatch() {
    let mut deps = mock_dependencies(&[]);

    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(10u128))],
        ),
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(10u128))],
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
    instantiate(deps.as_mut(), env, info, msg).unwrap();

    let simulation_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Simulation {
            offer_asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: "invalid".to_string(),
                },
                amount: Uint128::from(10u128),
            },
        },
    );

    assert_eq!(simulation_res.unwrap_err(), ContractError::AssetMismatch {});

    // check reverse simulation res
    let reverse_simulation_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::ReverseSimulation {
            ask_asset: Asset {
                info: AssetInfo::Token {
                    contract_addr: "invalid".to_string(),
                },
                amount: Uint128::from(10u128),
            },
        },
    );

    assert_eq!(
        reverse_simulation_res.unwrap_err(),
        ContractError::AssetMismatch {}
    );
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
