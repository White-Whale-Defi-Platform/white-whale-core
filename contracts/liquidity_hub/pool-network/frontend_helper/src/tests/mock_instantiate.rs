use cosmwasm_std::{to_json_binary, Addr, Decimal, Uint128, WasmMsg};
use cw20::Cw20Coin;
use cw_multi_test::{App, Executor};

use white_whale::{
    fee::Fee,
    pool_network::asset::{Asset, AssetInfo, PairType},
};

use crate::tests::mock_info::mock_creator;
use crate::tests::store_code::fee_distributor_mock_contract;

use super::{
    mock_info::mock_admin,
    store_code::{
        store_cw20_token_code, store_factory_code, store_frontend_helper, store_incentive,
        store_pair,
    },
};

pub struct AppInstantiateResponse {
    pub frontend_helper: Addr,
    pub pair_address: Addr,
    pub lp_token: AssetInfo,
    /// The returned pool assets.
    pub pool_assets: [AssetInfo; 2],
    pub incentive_factory: Addr,
}

pub fn app_mock_instantiate(app: &mut App, pool_assets: [AssetInfo; 2]) -> AppInstantiateResponse {
    let factory_id = store_factory_code(app);
    let token_id = store_cw20_token_code(app);
    let incentive_id = store_incentive(app);
    let frontend_helper_id = store_frontend_helper(app);
    let pair_id = store_pair(app);
    let fee_distributor_id = fee_distributor_mock_contract(app);

    // if the pair needs a token, create it and give the token to the user
    let pool_assets: [AssetInfo; 2] = pool_assets
        .into_iter()
        .map(|asset| match asset {
            AssetInfo::Token { contract_addr } => {
                let token_addr = app
                    .instantiate_contract(
                        token_id,
                        mock_admin().sender,
                        &cw20_base::msg::InstantiateMsg {
                            decimals: 6,
                            initial_balances: vec![Cw20Coin {
                                address: mock_creator().sender.into_string(),
                                amount: Uint128::new(10_000),
                            }],
                            name: contract_addr.clone(),
                            symbol: "mockToken".to_string(),
                            marketing: None,
                            mint: None,
                        },
                        &[],
                        contract_addr,
                        None,
                    )
                    .unwrap();

                AssetInfo::Token {
                    contract_addr: token_addr.into_string(),
                }
            }
            v => v,
        })
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    #[cfg(not(feature = "osmosis"))]
    let pool_fee = white_whale::pool_network::pair::PoolFee {
        burn_fee: Fee {
            share: Decimal::zero(),
        },
        protocol_fee: Fee {
            share: Decimal::zero(),
        },
        swap_fee: Fee {
            share: Decimal::zero(),
        },
    };

    #[cfg(feature = "osmosis")]
    let pool_fee = white_whale::pool_network::pair::PoolFee {
        burn_fee: Fee {
            share: Decimal::zero(),
        },
        protocol_fee: Fee {
            share: Decimal::zero(),
        },
        swap_fee: Fee {
            share: Decimal::zero(),
        },
        osmosis_fee: Fee {
            share: Decimal::zero(),
        },
    };

    let instantiate_msg = white_whale::pool_network::pair::InstantiateMsg {
        token_factory_lp: false,
        token_code_id: token_id,
        pool_fees: pool_fee.clone(),
        pair_type: PairType::ConstantProduct,
        fee_collector_addr: "fee_collector_addr".to_string(),
        asset_decimals: [6, 6],
        asset_infos: pool_assets.clone(),
    };

    // create the pair
    let pair = app
        .instantiate_contract(
            pair_id,
            mock_admin().sender,
            &instantiate_msg,
            &[],
            "mock pair",
            None,
        )
        .unwrap();

    // get the LP token
    let lp_token = app
        .wrap()
        .query_wasm_smart::<white_whale::pool_network::asset::PairInfo>(
            pair.clone(),
            &white_whale::pool_network::pair::QueryMsg::Pair {},
        )
        .unwrap()
        .liquidity_token;

    // create the fee distributor to use
    let fee_distributor = app
        .instantiate_contract(
            fee_distributor_id,
            mock_admin().sender,
            &fee_distributor_mock::msg::InstantiateMsg {},
            &[],
            "mock fee distributor",
            None,
        )
        .unwrap();

    let incentive_factory = app
        .instantiate_contract(
            factory_id,
            mock_admin().sender,
            &white_whale::pool_network::incentive_factory::InstantiateMsg {
                create_flow_fee: Asset {
                    amount: Uint128::zero(),
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                },
                fee_collector_addr: "fee_collector".to_string(),
                incentive_code_id: incentive_id,
                max_concurrent_flows: 7,
                max_flow_epoch_buffer: 100,
                max_unbonding_duration: 100000,
                min_unbonding_duration: 86400,
                fee_distributor_addr: fee_distributor.to_string(),
            },
            &[],
            "mock incentive factory",
            None,
        )
        .unwrap();

    app.execute(
        mock_admin().sender,
        WasmMsg::Execute {
            contract_addr: incentive_factory.to_string(),
            msg: to_json_binary(
                &white_whale::pool_network::incentive_factory::ExecuteMsg::CreateIncentive {
                    lp_asset: lp_token.clone(),
                },
            )
            .unwrap(),
            funds: vec![],
        }
        .into(),
    )
    .unwrap();

    let frontend_helper = app
        .instantiate_contract(
            frontend_helper_id,
            mock_admin().sender,
            &white_whale::pool_network::frontend_helper::InstantiateMsg {
                incentive_factory: incentive_factory.clone().into_string(),
            },
            &[],
            "frontend helper",
            None,
        )
        .unwrap();

    AppInstantiateResponse {
        frontend_helper,
        pair_address: pair,
        lp_token,
        pool_assets,
        incentive_factory,
    }
}
