use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{from_json, Addr, Uint64};
use cw_controllers::AdminError;

use epoch_manager::contract::{execute, query};
use epoch_manager::ContractError;
use white_whale::epoch_manager::epoch_manager::{
    ConfigResponse, EpochConfig, ExecuteMsg, QueryMsg,
};
use white_whale::pool_network::mock_querier::mock_dependencies;

use crate::common::mock_instantiation;

mod common;

#[test]
fn update_config_successfully() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("owner", &[]);
    let current_time = mock_env().block.time;
    mock_instantiation(deps.as_mut(), info.clone()).unwrap();

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_json(&query_res).unwrap();
    assert_eq!(
        EpochConfig {
            duration: Uint64::new(86400),
            genesis_epoch: Uint64::new(current_time.nanos()),
        },
        config_res.epoch_config
    );
    assert_eq!(Addr::unchecked("owner"), config_res.owner);

    let msg = ExecuteMsg::UpdateConfig {
        owner: Some("new_owner".to_string()),
        epoch_config: Some(EpochConfig {
            duration: Uint64::new(172800),
            genesis_epoch: Uint64::new(current_time.nanos()),
        }),
    };

    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_json(&query_res).unwrap();
    assert_eq!(
        EpochConfig {
            duration: Uint64::new(172800),
            genesis_epoch: Uint64::new(current_time.nanos()),
        },
        config_res.epoch_config
    );
    assert_eq!(Addr::unchecked("new_owner"), config_res.owner);
}

#[test]
fn update_config_unsuccessfully() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("owner", &[]);
    let current_time = mock_env().block.time;
    mock_instantiation(deps.as_mut(), info.clone()).unwrap();

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_json(&query_res).unwrap();
    assert_eq!(
        EpochConfig {
            duration: Uint64::new(86400),
            genesis_epoch: Uint64::new(current_time.nanos()),
        },
        config_res.epoch_config
    );
    assert_eq!(Addr::unchecked("owner"), config_res.owner);

    let msg = ExecuteMsg::UpdateConfig {
        owner: Some("new_owner".to_string()),
        epoch_config: Some(EpochConfig {
            duration: Uint64::new(172800),
            genesis_epoch: Uint64::new(current_time.nanos()),
        }),
    };

    let info = mock_info("unauthorized", &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

    match err {
        ContractError::AdminError(error) => {
            assert_eq!(error, AdminError::NotAdmin {})
        }
        _ => panic!("should return ContractError::AdminError(AdminError::NotAdmin)"),
    }

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_json(&query_res).unwrap();

    // has not changed
    assert_eq!(
        EpochConfig {
            duration: Uint64::new(86400),
            genesis_epoch: Uint64::new(current_time.nanos()),
        },
        config_res.epoch_config
    );
    assert_eq!(Addr::unchecked("owner"), config_res.owner);
}
