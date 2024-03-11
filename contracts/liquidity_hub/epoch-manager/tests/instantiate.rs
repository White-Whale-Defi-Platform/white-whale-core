use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{from_json, Addr, Uint64};

use epoch_manager::contract::{instantiate, query};
use epoch_manager::ContractError;
use white_whale_std::epoch_manager::epoch_manager::{
    ConfigResponse, EpochConfig, EpochV2, InstantiateMsg, QueryMsg,
};
use white_whale_std::pool_network::mock_querier::mock_dependencies;

mod common;

#[test]
fn instantiation_successful() {
    let mut deps = mock_dependencies(&[]);

    let current_time = mock_env().block.time;
    let info = mock_info("owner", &[]);
    let msg = InstantiateMsg {
        start_epoch: EpochV2 {
            id: 123,
            start_time: current_time,
        },
        epoch_config: EpochConfig {
            duration: Uint64::new(86400),
            genesis_epoch: Uint64::new(current_time.nanos()),
        },
    };

    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_json(query_res).unwrap();
    assert_eq!(
        EpochConfig {
            duration: Uint64::new(86400),
            genesis_epoch: Uint64::new(current_time.nanos()),
        },
        config_res.epoch_config
    );
    assert_eq!(Addr::unchecked("owner"), config_res.owner);
}

#[test]
fn instantiation_unsuccessful() {
    let mut deps = mock_dependencies(&[]);

    let current_time = mock_env().block.time;
    let info = mock_info("owner", &[]);
    let msg = InstantiateMsg {
        start_epoch: EpochV2 {
            id: 123,
            start_time: current_time.minus_days(1),
        },
        epoch_config: EpochConfig {
            duration: Uint64::new(86400),
            genesis_epoch: Uint64::new(current_time.minus_days(1).nanos()),
        },
    };

    let err = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
    match err {
        ContractError::InvalidStartTime => {}
        _ => panic!("should return ContractError::InvalidStartTime"),
    }

    let msg = InstantiateMsg {
        start_epoch: EpochV2 {
            id: 123,
            start_time: current_time.plus_days(1),
        },
        epoch_config: EpochConfig {
            duration: Uint64::new(86400),
            genesis_epoch: Uint64::new(current_time.plus_days(2).nanos()),
        },
    };

    let err = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    match err {
        ContractError::EpochConfigMismatch => {}
        _ => panic!("should return ContractError::EpochConfigMismatch"),
    }
}
