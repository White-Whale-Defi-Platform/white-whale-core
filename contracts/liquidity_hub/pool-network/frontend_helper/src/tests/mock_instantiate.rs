use cosmwasm_std::{to_binary, Addr, Decimal, Uint128, WasmMsg};
use cw_multi_test::{App, Executor};
use white_whale::{
    fee::Fee,
    pool_network::asset::{Asset, AssetInfo, PairType},
};

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
}

pub fn app_mock_instantiate(app: &mut App) -> AppInstantiateResponse {
    let factory_id = store_factory_code(app);
    let token_id = store_cw20_token_code(app);
    let incentive_id = store_incentive(app);
    let frontend_helper_id = store_frontend_helper(app);
    let pair_id = store_pair(app);

    // create the pair
    let pair = app
        .instantiate_contract(
            pair_id,
            mock_admin().sender,
            &white_whale::pool_network::pair::InstantiateMsg {
                token_factory_lp: false,
                token_code_id: token_id,
                pool_fees: white_whale::pool_network::pair::PoolFee {
                    burn_fee: Fee {
                        share: Decimal::zero(),
                    },
                    protocol_fee: Fee {
                        share: Decimal::zero(),
                    },
                    swap_fee: Fee {
                        share: Decimal::zero(),
                    },
                },
                pair_type: PairType::ConstantProduct,
                fee_collector_addr: "fee_collector_addr".to_string(),
                asset_decimals: [6, 6],
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: "token_a".to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: "token_b".to_string(),
                    },
                ],
            },
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

    let incentive_factory = app
        .instantiate_contract(
            factory_id,
            mock_admin().sender,
            &white_whale::pool_network::incentive_factory::InstantiateMsg {
                create_flow_fee: Asset {
                    amount: Uint128::zero(),
                    info: AssetInfo::NativeToken {
                        denom: "ust".to_string(),
                    },
                },
                fee_collector_addr: "fee_collector".to_string(),
                incentive_contract_id: incentive_id,
                max_concurrent_flows: 7,
                max_flow_start_time_buffer: 100,
                max_unbonding_duration: 100000,
                min_unbonding_duration: 86400,
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
            msg: to_binary(
                &white_whale::pool_network::incentive_factory::ExecuteMsg::CreateIncentive {
                    lp_address: lp_token.clone(),
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
    }
}
