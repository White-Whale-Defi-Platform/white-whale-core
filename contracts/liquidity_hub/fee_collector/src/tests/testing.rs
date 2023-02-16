use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{from_binary, Addr, DepsMut, MessageInfo, Response};
use cw2::{get_contract_version, ContractVersion};
use pool_network::asset::AssetInfo;
use std::env;

use crate::contract::{execute, instantiate, migrate, query};
use pool_network::mock_querier::mock_dependencies;

use crate::msg::ExecuteMsg::AggregateFees;
use crate::msg::{ExecuteMsg, FeesFor, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::ConfigResponse;
use crate::ContractError;

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
fn collect_fees_unsuccessfully_unauthorized() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("owner", &[]);
    mock_instantiation(deps.as_mut(), info).unwrap();

    // unauthorized tries collecting fees
    let info = mock_info("unauthorized", &[]);
    let msg = ExecuteMsg::CollectFees {
        collect_fees_for: FeesFor::Contracts { contracts: vec![] },
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

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(config_res.owner, Addr::unchecked("owner"));

    let msg = ExecuteMsg::UpdateConfig {
        owner: Some("new_owner".to_string()),
        pool_router: Some("new_router".to_string()),
    };

    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(config_res.owner, Addr::unchecked("new_owner"));
    assert_eq!(config_res.pool_router, Addr::unchecked("new_router"));
}

#[test]
fn test_update_config_unsuccessfully_unauthorized() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("owner", &[]);
    mock_instantiation(deps.as_mut(), info).unwrap();

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(config_res.owner, Addr::unchecked("owner"));

    let info = mock_info("unauthorized", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: Some("new_owner".to_string()),
        pool_router: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    match res {
        Ok(_) => panic!("should return ContractError::Unauthorized"),
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("should return ContractError::Unauthorized"),
    }
}

#[test]
fn test_migration() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("owner", &[]);
    mock_instantiation(deps.as_mut(), info).unwrap();

    assert_eq!(
        get_contract_version(&deps.storage),
        Ok(ContractVersion {
            contract: "white_whale-fee_collector".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string()
        })
    );

    let res = migrate(deps.as_mut(), mock_env(), MigrateMsg {});

    // should not be able to migrate as the version is lower
    match res {
        Err(ContractError::MigrateInvalidVersion { .. }) => (),
        _ => panic!("should return ContractError::MigrateInvalidVersion"),
    }
}

#[test]
fn test_aggregate_fee_for_contracts_err() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("owner", &[]);
    mock_instantiation(deps.as_mut(), info.clone()).unwrap();

    // should error, can't collect fees for contracts
    let msg = AggregateFees {
        asset_info: AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        },
        aggregate_fees_for: FeesFor::Contracts { contracts: vec![] },
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    match res {
        Ok(_) => panic!("should return ContractError::InvalidContractsFeeAggregation"),
        Err(ContractError::InvalidContractsFeeAggregation {}) => (),
        _ => panic!("should return ContractError::InvalidContractsFeeAggregation"),
    }
}
