#[cfg(feature = "token_factory")]
use crate::state::LP_SYMBOL;
use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    attr, to_binary, Coin, CosmosMsg, Decimal, Reply, ReplyOn, Response, StdError, SubMsg,
    SubMsgResponse, SubMsgResult, Uint128, WasmMsg,
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
fn try_native_to_token() {
    let total_share = Uint128::from(30000000000u128);
    let asset_pool_amount = Uint128::from(20000000000u128);
    let collateral_pool_amount = Uint128::from(30000000000u128);
    let exchange_rate: Decimal = Decimal::from_ratio(asset_pool_amount, collateral_pool_amount);
    let offer_amount = Uint128::from(1500000000u128);

    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: collateral_pool_amount + offer_amount,
        /* user deposit must be pre-applied */
    }]);

    // Note: In order to support and write tests sooner I override the MockAPI in mock_querier with a SimpleMockAPI
    // The effect is I can continue testing with instantiate2 but now an addr like liquidity0000 becomes TEMP_CONTRACT_ADDR
    // This likely breaks tests elsewhere
    let TEMP_CONTRACT_ADDR: String =
        "contract757573642d6d4141504c61646472303030303132333435".to_string();

    deps.querier.with_token_balances(&[
        (
            &TEMP_CONTRACT_ADDR.clone(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &total_share)],
        ),
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &asset_pool_amount)],
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
        AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        AssetInfo::Token {
            contract_addr: "asset0000".to_string(),
        },
    ];

    let msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos.to_vec(),
        pool_fees: PoolFee {
            protocol_fee: Fee {
                share: Decimal::from_ratio(1u128, 1000u128),
            },
            swap_fee: Fee {
                share: Decimal::from_ratio(3u128, 1000u128),
            },
            burn_fee: Fee {
                share: Decimal::from_ratio(1u128, 1000u128),
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

    // normal swap
    let msg = ExecuteMsg::Swap {
        offer_asset: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: offer_amount,
        },
        ask_asset: AssetInfo::Token {
            contract_addr: "asset0000".to_string(),
        },
        belief_price: None,
        max_spread: Some(Decimal::percent(5)),
        to: None,
        pair_identifier: "0".to_string(),
    };
    let env = mock_env();
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: offer_amount,
        }],
    );
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    println!("{:?}", res);
    assert_eq!(res.messages.len(), 1);

    let msg_transfer = res.messages.get(0).expect("no message");

    // current price is 1.5, so expected return without spread is 1000
    // ask_amount = ((ask_pool - accrued protocol fees) * offer_amount / (offer_pool - accrued protocol fees + offer_amount))
    // 952.380952 = (20000 - 0) * 1500 / (30000 - 0 + 1500) - swap_fee - protocol_fee - burn_fee
    // TODO: Returned amount is 904545457 and spread is up to 5% -- 43290043. Investigate this
    let expected_ret_amount = Uint128::from(952_380_952u128);
    let expected_spread_amount = (offer_amount * exchange_rate)
        .checked_sub(expected_ret_amount)
        .unwrap();
    let expected_swap_fee_amount = expected_ret_amount.multiply_ratio(3u128, 1000u128); // 0.3%
    let expected_protocol_fee_amount = expected_ret_amount.multiply_ratio(1u128, 1000u128); // 0.1%
    let expected_burn_fee_amount = expected_ret_amount.multiply_ratio(1u128, 1000u128); // 0.1%
    let expected_return_amount = expected_ret_amount
        .checked_sub(expected_swap_fee_amount)
        .unwrap()
        .checked_sub(expected_protocol_fee_amount)
        .unwrap()
        .checked_sub(expected_burn_fee_amount)
        .unwrap();

    // since there is a burn_fee on the PoolFee, check burn message
    // since we swapped to a cw20 token, the burn message should be a Cw20ExecuteMsg::Burn
    let expected_burn_msg = SubMsg {
        id: 0,
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset0000".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Burn {
                amount: expected_burn_fee_amount,
            })
            .unwrap(),
            funds: vec![],
        }),
        gas_limit: None,
        reply_on: ReplyOn::Never,
    };
    println!("{:?}", exchange_rate);
    println!("{:?}", offer_amount * exchange_rate);
    assert_eq!(res.messages.last().unwrap().clone(), expected_burn_msg);

    // // as we swapped native to token, we accumulate the protocol fees in token
    // let protocol_fees_for_token = query_fees(
    //     deps.as_ref(),
    //     Some("asset0000".to_string()),
    //     None,
    //     COLLECTED_PROTOCOL_FEES,
    //     Some(ALL_TIME_COLLECTED_PROTOCOL_FEES),
    // )
    // .unwrap()
    // .fees;
    // assert_eq!(
    //     protocol_fees_for_token.first().unwrap().amount,
    //     expected_protocol_fee_amount
    // );
    // let burned_fees_for_token = query_fees(
    //     deps.as_ref(),
    //     Some("asset0000".to_string()),
    //     None,
    //     ALL_TIME_BURNED_FEES,
    //     None,
    // )
    // .unwrap()
    // .fees;
    // assert_eq!(
    //     burned_fees_for_token.first().unwrap().amount,
    //     expected_burn_fee_amount
    // );
    // let protocol_fees_for_native = query_fees(
    //     deps.as_ref(),
    //     Some("uusd".to_string()),
    //     None,
    //     COLLECTED_PROTOCOL_FEES,
    //     Some(ALL_TIME_COLLECTED_PROTOCOL_FEES),
    // )
    // .unwrap()
    // .fees;
    // assert_eq!(
    //     protocol_fees_for_native.first().unwrap().amount,
    //     Uint128::zero()
    // );
    // let burned_fees_for_native = query_fees(
    //     deps.as_ref(),
    //     Some("uusd".to_string()),
    //     None,
    //     ALL_TIME_BURNED_FEES,
    //     None,
    // )
    // .unwrap()
    // .fees;
    // assert_eq!(
    //     burned_fees_for_native.first().unwrap().amount,
    //     Uint128::zero()
    // );

    // // check simulation res, reset values pre-swap to check simulation
    // deps.querier.with_balance(&[(
    //     &MOCK_CONTRACT_ADDR.to_string(),
    //     vec![Coin {
    //         denom: "uusd".to_string(),
    //         amount: collateral_pool_amount,
    //         /* user deposit must be pre-applied */
    //     }],
    // )]);

    // // reset protocol fees so the simulation returns same values as the actual swap
    // COLLECTED_PROTOCOL_FEES
    //     .save(
    //         &mut deps.storage,
    //         &vec![
    //             Asset {
    //                 info: AssetInfo::NativeToken {
    //                     denom: "uusd".to_string(),
    //                 },
    //                 amount: Uint128::zero(),
    //             },
    //             Asset {
    //                 info: AssetInfo::Token {
    //                     contract_addr: "asset0000".to_string(),
    //                 },
    //                 amount: Uint128::zero(),
    //             },
    //         ],
    //     )
    //     .unwrap();

    // let simulation_res: SimulationResponse = from_binary(
    //     &query(
    //         deps.as_ref(),
    //         mock_env(),
    //         QueryMsg::Simulation {
    //             offer_asset: Asset {
    //                 info: AssetInfo::NativeToken {
    //                     denom: "uusd".to_string(),
    //                 },
    //                 amount: offer_amount,
    //             },
    //         },
    //     )
    //     .unwrap(),
    // )
    // .unwrap();

    // assert_eq!(expected_return_amount, simulation_res.return_amount);
    // assert_eq!(expected_swap_fee_amount, simulation_res.swap_fee_amount);
    // assert_eq!(expected_burn_fee_amount, simulation_res.burn_fee_amount);
    // assert_eq!(expected_spread_amount, simulation_res.spread_amount);
    // assert_eq!(
    //     expected_protocol_fee_amount,
    //     simulation_res.protocol_fee_amount
    // );

    // // reset protocol fees so the simulation returns same values as the actual swap
    // COLLECTED_PROTOCOL_FEES
    //     .save(
    //         &mut deps.storage,
    //         &vec![
    //             Asset {
    //                 info: AssetInfo::NativeToken {
    //                     denom: "uusd".to_string(),
    //                 },
    //                 amount: Uint128::zero(),
    //             },
    //             Asset {
    //                 info: AssetInfo::Token {
    //                     contract_addr: "asset0000".to_string(),
    //                 },
    //                 amount: Uint128::zero(),
    //             },
    //         ],
    //     )
    //     .unwrap();

    // let reverse_simulation_res: ReverseSimulationResponse = from_binary(
    //     &query(
    //         deps.as_ref(),
    //         mock_env(),
    //         QueryMsg::ReverseSimulation {
    //             ask_asset: Asset {
    //                 info: AssetInfo::Token {
    //                     contract_addr: "asset0000".to_string(),
    //                 },
    //                 amount: expected_return_amount,
    //             },
    //         },
    //     )
    //     .unwrap(),
    // )
    // .unwrap();

    // assert!(
    //     (offer_amount.u128() as i128 - reverse_simulation_res.offer_amount.u128() as i128).abs()
    //         < 3i128
    // );
    // assert!(
    //     (expected_swap_fee_amount.u128() as i128
    //         - reverse_simulation_res.swap_fee_amount.u128() as i128)
    //         .abs()
    //         < 3i128
    // );
    // assert!(
    //     (expected_spread_amount.u128() as i128
    //         - reverse_simulation_res.spread_amount.u128() as i128)
    //         .abs()
    //         < 3i128
    // );
    // assert!(
    //     (expected_protocol_fee_amount.u128() as i128
    //         - reverse_simulation_res.protocol_fee_amount.u128() as i128)
    //         .abs()
    //         < 3i128
    // );
    // assert!(
    //     (expected_burn_fee_amount.u128() as i128
    //         - reverse_simulation_res.burn_fee_amount.u128() as i128)
    //         .abs()
    //         < 3i128
    // );

    // assert_eq!(
    //     res.attributes,
    //     vec![
    //         attr("action", "swap"),
    //         attr("sender", "addr0000"),
    //         attr("receiver", "addr0000"),
    //         attr("offer_asset", "uusd"),
    //         attr("ask_asset", "asset0000"),
    //         attr("offer_amount", offer_amount.to_string()),
    //         attr("return_amount", expected_return_amount.to_string()),
    //         attr("spread_amount", expected_spread_amount.to_string()),
    //         attr("swap_fee_amount", expected_swap_fee_amount.to_string()),
    //         attr(
    //             "protocol_fee_amount",
    //             expected_protocol_fee_amount.to_string(),
    //         ),
    //         attr("burn_fee_amount", expected_burn_fee_amount.to_string()),
    //         attr("swap_type", "ConstantProduct"),
    //     ]
    // );

    // assert_eq!(
    //     &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
    //         contract_addr: "asset0000".to_string(),
    //         msg: to_binary(&Cw20ExecuteMsg::Transfer {
    //             recipient: "addr0000".to_string(),
    //             amount: expected_return_amount,
    //         })
    //         .unwrap(),
    //         funds: vec![],
    //     })),
    //     msg_transfer,
    // );
}
