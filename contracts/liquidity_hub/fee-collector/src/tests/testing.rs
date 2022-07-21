use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{from_binary, Addr, DepsMut, MessageInfo, Response};

use crate::contract::{execute, instantiate, query};
use terraswap::mock_querier::mock_dependencies;

use crate::msg::{ExecuteMsg, FactoriesResponse, InstantiateMsg, QueryMsg};
use crate::state::ConfigResponse;
use crate::ContractError;
use crate::queries::query_config;

pub fn mock_instantiation(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let msg = InstantiateMsg {};
    instantiate(deps, mock_env(), info, msg)
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {};
    let info = mock_info("owner", &[]);

    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!("owner".to_string(), config_res.owner);
}

#[test]
fn add_factory_successful() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("owner", &[]);
    mock_instantiation(deps.as_mut(), info.clone()).unwrap();

    let msg = ExecuteMsg::AddFactory {
        factory_addr: "factory1".to_string(),
    };

    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Factories {
            start_after: None,
            limit: None,
        },
    )
    .unwrap();
    let factories_response: FactoriesResponse = from_binary(&query_res).unwrap();
    assert_eq!(1usize, factories_response.factories.len());
    assert_eq!(factories_response.factories[0], Addr::unchecked("factory1"));
}

#[test]
fn add_factory_unsuccessful_unauthorized() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("owner", &[]);
    mock_instantiation(deps.as_mut(), info).unwrap();

    let msg = ExecuteMsg::AddFactory {
        factory_addr: "factory1".to_string(),
    };

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("unauthorized", &[]),
        msg,
    );

    match res {
        Ok(_) => panic!("should return ContractError::Unauthorized"),
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("should return ContractError::Unauthorized"),
    }
}

#[test]
fn remove_factory_successful() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("owner", &[]);
    mock_instantiation(deps.as_mut(), info.clone()).unwrap();

    //add factory first
    let msg = ExecuteMsg::AddFactory {
        factory_addr: "factory1".to_string(),
    };
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Factories {
            start_after: None,
            limit: None,
        },
    )
    .unwrap();
    let factories_response: FactoriesResponse = from_binary(&query_res).unwrap();
    assert_eq!(1usize, factories_response.factories.len());
    assert_eq!(factories_response.factories[0], Addr::unchecked("factory1"));

    //remove factory
    let msg = ExecuteMsg::RemoveFactory {
        factory_addr: "factory1".to_string(),
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Factories {
            start_after: None,
            limit: None,
        },
    )
    .unwrap();
    let factories_response: FactoriesResponse = from_binary(&query_res).unwrap();
    assert!(factories_response.factories.is_empty());
}

#[test]
fn remove_factory_unsuccessful_unauthorized() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("owner", &[]);
    mock_instantiation(deps.as_mut(), info.clone()).unwrap();

    //add factory first
    let msg = ExecuteMsg::AddFactory {
        factory_addr: "factory1".to_string(),
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Factories {
            start_after: None,
            limit: None,
        },
    )
    .unwrap();
    let factories_response: FactoriesResponse = from_binary(&query_res).unwrap();
    assert_eq!(1usize, factories_response.factories.len());
    assert_eq!(factories_response.factories[0], Addr::unchecked("factory1"));

    //remove factory
    let msg = ExecuteMsg::RemoveFactory {
        factory_addr: "factory1".to_string(),
    };
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("unauthorized", &[]),
        msg,
    );

    match res {
        Ok(_) => panic!("should return ContractError::Unauthorized"),
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("should return ContractError::Unauthorized"),
    }
}

#[test]
fn remove_unknown_factory() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("owner", &[]);
    mock_instantiation(deps.as_mut(), info.clone()).unwrap();

    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Factories {
            start_after: None,
            limit: None,
        },
    )
    .unwrap();
    let factories_response: FactoriesResponse = from_binary(&query_res).unwrap();
    assert!(factories_response.factories.is_empty());

    //try removing an unknown factory
    let msg = ExecuteMsg::RemoveFactory {
        factory_addr: "unknown factory".to_string(),
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Factories {
            start_after: None,
            limit: None,
        },
    )
    .unwrap();
    let factories_response: FactoriesResponse = from_binary(&query_res).unwrap();
    assert!(factories_response.factories.is_empty());
}

#[test]
fn collect_fees_unsuccessfully_unauthorized() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("owner", &[]);
    mock_instantiation(deps.as_mut(), info.clone()).unwrap();

    // unauthorized tries collecting fees
    let info = mock_info("unauthorized", &[]);
    let msg = ExecuteMsg::CollectFees {
        factory_addr: None,
        contracts: None,
        start_after: None,
        limit: None
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    match res {
        Ok(_) => panic!("should return ContractError::Unauthorized"),
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("should return ContractError::Unauthorized"),
    }
}

#[test]
fn test_update_config_successfully() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("owner", &[]);
    mock_instantiation(deps.as_mut(), info.clone()).unwrap();

    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Config {},
    )
        .unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(config_res.owner, Addr::unchecked("owner"));

    let msg = ExecuteMsg::UpdateConfig {
        owner: Some("new_owner".to_string()),
    };

    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Config {},
    )
        .unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(config_res.owner, Addr::unchecked("new_owner"));
}


#[test]
fn test_update_config_unsuccessfully_unauthorized() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("owner", &[]);
    mock_instantiation(deps.as_mut(), info.clone()).unwrap();

    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Config {},
    )
        .unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(config_res.owner, Addr::unchecked("owner"));

    let info = mock_info("unauthorized", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: Some("new_owner".to_string()),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    match res {
        Ok(_) => panic!("should return ContractError::Unauthorized"),
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("should return ContractError::Unauthorized"),
    }
}

