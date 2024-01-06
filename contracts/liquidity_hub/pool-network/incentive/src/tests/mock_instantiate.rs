use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage},
    to_json_binary, Addr, Env, OwnedDeps, Uint128, Uint64, WasmMsg,
};
use cw20::Cw20Coin;
use cw_multi_test::{App, Executor};
use white_whale::epoch_manager::epoch_manager::EpochConfig;
use white_whale::pool_network::asset::{Asset, AssetInfo};

use crate::contract::instantiate;

use super::{
    mock_creator,
    mock_info::mock_admin,
    store_code::{
        fee_distributor_mock_contract, store_cw20_token_code, store_factory_code, store_incentive,
    },
};

pub fn mock_instantiate() -> (OwnedDeps<MockStorage, MockApi, MockQuerier>, Env) {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let creator = mock_creator();

    instantiate(
        deps.as_mut(),
        env.clone(),
        creator,
        white_whale::pool_network::incentive::InstantiateMsg {
            lp_asset: AssetInfo::NativeToken {
                denom: "lp_addr".to_string(),
            },
            fee_distributor_address: "fee_distributor_address".to_string(),
        },
    )
    .unwrap();

    (deps, env)
}

pub struct AppInstantiateResponse {
    pub incentive_addr: Addr,
    pub lp_addr: Addr,
}

pub fn app_mock_instantiate(app: &mut App, lp_balance: Uint128) -> AppInstantiateResponse {
    let factory_id = store_factory_code(app);
    let token_id = store_cw20_token_code(app);
    let incentive_id = store_incentive(app);
    let fee_distributor_id = fee_distributor_mock_contract(app);

    // create the LP token to use
    let lp_addr = app
        .instantiate_contract(
            token_id,
            mock_admin().sender,
            &cw20_base::msg::InstantiateMsg {
                name: "mock_lp".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: mock_creator().sender.into_string(),
                    amount: lp_balance,
                }],
                marketing: None,
                mint: None,
                symbol: "uMock".to_string(),
            },
            &[],
            "mock LP token",
            None,
        )
        .unwrap();

    let lp_addr_token = AssetInfo::Token {
        contract_addr: lp_addr.to_string(),
    };

    // create the fee distributor to use
    let fee_distributor = app
        .instantiate_contract(
            fee_distributor_id,
            mock_admin().sender,
            &white_whale::fee_distributor::InstantiateMsg {
                bonding_contract_addr: "bonding_contract_addr".to_string(),
                fee_collector_addr: "fee_collector_addr".to_string(),
                grace_period: Uint64::one(),
                epoch_config: EpochConfig {
                    duration: Uint64::new(86400_000000000u64),
                    genesis_epoch: Uint64::new(1_685_458_800_000_000_000_u64),
                },
                distribution_asset: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
            },
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
                        denom: "ust".to_string(),
                    },
                },
                fee_collector_addr: "fee_collector".to_string(),
                incentive_code_id: incentive_id,
                max_concurrent_flows: 7,
                max_flow_epoch_buffer: 100,
                max_unbonding_duration: 31556926,
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
                    lp_asset: AssetInfo::Token {
                        contract_addr: lp_addr.to_string(),
                    },
                },
            )
            .unwrap(),
            funds: vec![],
        }
        .into(),
    )
    .unwrap();

    let incentive_addr: white_whale::pool_network::incentive_factory::IncentiveResponse = app
        .wrap()
        .query_wasm_smart(
            incentive_factory,
            &white_whale::pool_network::incentive_factory::QueryMsg::Incentive {
                lp_asset: lp_addr_token,
            },
        )
        .unwrap();

    AppInstantiateResponse {
        incentive_addr: incentive_addr.expect("No incentive contract existed"),
        lp_addr,
    }
}
