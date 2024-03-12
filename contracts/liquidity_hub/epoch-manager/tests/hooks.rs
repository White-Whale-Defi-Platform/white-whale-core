use cosmwasm_std::from_json;
use cosmwasm_std::testing::{mock_env, mock_info};
use cw_controllers::{AdminError, HookError, HooksResponse};

use crate::common::{mock_add_hook, mock_instantiation};
use epoch_manager::contract::{execute, query};
use epoch_manager::ContractError;
use white_whale_std::epoch_manager::epoch_manager::{ExecuteMsg, QueryMsg};
use white_whale_std::pool_network::mock_querier::mock_dependencies;

mod common;
#[test]
fn add_hook_successfully() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("owner", &[]);
    mock_instantiation(deps.as_mut(), info.clone()).unwrap();

    let msg = ExecuteMsg::AddHook {
        contract_addr: "hook_contract_1".to_string(),
    };

    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Hook {
            hook: "hook_contract_1".to_string(),
        },
    )
    .unwrap();
    let hook_registered: bool = from_json(query_res).unwrap();
    assert!(!hook_registered);

    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Hook {
            hook: "hook_contract_1".to_string(),
        },
    )
    .unwrap();
    let hook_registered: bool = from_json(query_res).unwrap();
    assert!(hook_registered);

    for i in 2..10 {
        let msg = ExecuteMsg::AddHook {
            contract_addr: format!("hook_contract_{}", i),
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    }

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Hooks {}).unwrap();
    let hooks_response: HooksResponse = from_json(query_res).unwrap();
    assert_eq!(
        hooks_response,
        HooksResponse {
            hooks: vec![
                "hook_contract_1".to_string(),
                "hook_contract_2".to_string(),
                "hook_contract_3".to_string(),
                "hook_contract_4".to_string(),
                "hook_contract_5".to_string(),
                "hook_contract_6".to_string(),
                "hook_contract_7".to_string(),
                "hook_contract_8".to_string(),
                "hook_contract_9".to_string(),
            ]
        }
    );
}

#[test]
fn add_hook_unsuccessfully() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("owner", &[]);
    mock_instantiation(deps.as_mut(), info.clone()).unwrap();

    let msg = ExecuteMsg::AddHook {
        contract_addr: "hook_contract_1".to_string(),
    };

    let info = mock_info("unauthorized", &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    match err {
        ContractError::HookError(error) => {
            assert_eq!(error, HookError::Admin(AdminError::NotAdmin {}))
        }
        _ => panic!("should return ContractError::HookError::Admin(AdminError::NotAdmin)"),
    }
}

#[test]
fn remove_hook_successfully() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("owner", &[]);
    mock_instantiation(deps.as_mut(), info.clone()).unwrap();
    mock_add_hook(deps.as_mut(), info.clone()).unwrap();

    let msg = ExecuteMsg::RemoveHook {
        contract_addr: "hook_contract_1".to_string(),
    };

    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Hook {
            hook: "hook_contract_1".to_string(),
        },
    )
    .unwrap();
    let hook_registered: bool = from_json(query_res).unwrap();
    assert!(hook_registered);

    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Hook {
            hook: "hook_contract_1".to_string(),
        },
    )
    .unwrap();
    let hook_registered: bool = from_json(query_res).unwrap();
    assert!(!hook_registered);
}

#[test]
fn remove_hook_unsuccessfully() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("owner", &[]);
    mock_instantiation(deps.as_mut(), info.clone()).unwrap();
    mock_add_hook(deps.as_mut(), info).unwrap();

    let msg = ExecuteMsg::RemoveHook {
        contract_addr: "hook_contract_1".to_string(),
    };

    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Hook {
            hook: "hook_contract_1".to_string(),
        },
    )
    .unwrap();
    let hook_registered: bool = from_json(query_res).unwrap();
    assert!(hook_registered);

    let info = mock_info("unauthorized", &[]);

    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    match err {
        ContractError::HookError(error) => {
            assert_eq!(error, HookError::Admin(AdminError::NotAdmin {}))
        }
        _ => panic!("should return ContractError::HookError::Admin(AdminError::NotAdmin)"),
    }
}
