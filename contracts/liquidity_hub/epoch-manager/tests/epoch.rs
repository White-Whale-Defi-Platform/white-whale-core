use cosmwasm_std::from_json;
use cosmwasm_std::testing::{mock_env, mock_info};

use epoch_manager::contract::{execute, query};
use epoch_manager::ContractError;
use white_whale_std::epoch_manager::epoch_manager::{Epoch, EpochResponse, ExecuteMsg, QueryMsg};
use white_whale_std::epoch_manager::hooks::EpochChangedHookMsg;
use white_whale_std::pool_network::mock_querier::mock_dependencies;

use crate::common::{mock_add_hook, mock_instantiation};

mod common;

#[test]
fn create_new_epoch_successfully() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("owner", &[]);
    let mut env = mock_env();
    mock_instantiation(deps.as_mut(), info.clone()).unwrap();
    mock_add_hook(deps.as_mut(), info.clone()).unwrap();
    let next_epoch_time = env.block.time.plus_nanos(86400); //86400 is the duration of the epoch

    // move time ahead so we can create the epoch
    env.block.time = env.block.time.plus_nanos(86400);

    let msg = ExecuteMsg::CreateEpoch;
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::CurrentEpoch {}).unwrap();
    let epoch_response: EpochResponse = from_json(query_res).unwrap();

    let current_epoch = Epoch {
        id: 124,
        start_time: next_epoch_time,
    };

    assert_eq!(epoch_response.epoch, current_epoch);
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0].msg,
        EpochChangedHookMsg {
            current_epoch: current_epoch.clone()
        }
        .into_cosmos_msg("hook_contract_1")
        .unwrap()
    );

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Epoch { id: 124 }).unwrap();
    let epoch_response: EpochResponse = from_json(query_res).unwrap();

    assert_eq!(epoch_response.epoch, current_epoch);

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Epoch { id: 123 }).unwrap();
    let epoch_response: EpochResponse = from_json(query_res).unwrap();

    assert_eq!(
        epoch_response.epoch,
        Epoch {
            id: 123,
            start_time: next_epoch_time.minus_nanos(86400),
        }
    );
}

#[test]
fn create_new_epoch_unsuccessfully() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("owner", &[]);
    let mut env = mock_env();
    mock_instantiation(deps.as_mut(), info.clone()).unwrap();

    // move time ahead but not enough so the epoch creation fails
    env.block.time = env.block.time.plus_nanos(86300);

    let msg = ExecuteMsg::CreateEpoch;
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    match err {
        ContractError::CurrentEpochNotExpired => {}
        _ => panic!("should return ContractError::CurrentEpochNotExpired"),
    }
}
