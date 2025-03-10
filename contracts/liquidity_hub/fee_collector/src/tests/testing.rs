use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{from_json, Addr, Decimal, DepsMut, MessageInfo, Response};
use cw2::{get_contract_version, set_contract_version, ContractVersion};
use std::env;
use std::str::FromStr;

use crate::contract::{execute, instantiate, migrate, query};
use white_whale_std::pool_network::mock_querier::mock_dependencies;

use crate::ContractError;
use white_whale_std::fee_collector::{Config, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

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
    let config_res: Config = from_json(&query_res).unwrap();
    assert_eq!("owner".to_string(), config_res.owner);
}

#[test]
fn test_update_config_successfully() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("owner", &[]);
    mock_instantiation(deps.as_mut(), info.clone()).unwrap();

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: Config = from_json(&query_res).unwrap();
    assert_eq!(config_res.owner, Addr::unchecked("owner"));

    let msg = ExecuteMsg::UpdateConfig {
        owner: Some("new_owner".to_string()),
        pool_router: Some("new_router".to_string()),
        fee_distributor: None,
        pool_factory: None,
        vault_factory: None,
        take_rate: Some(Decimal::from_str("0.5").unwrap()),
        take_rate_dao_address: Some("take_rate_dao_address".to_string()),
        is_take_rate_active: Some(true),
    };

    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: Config = from_json(&query_res).unwrap();
    assert_eq!(config_res.owner, Addr::unchecked("new_owner"));
    assert_eq!(config_res.pool_router, Addr::unchecked("new_router"));
    assert_eq!(
        config_res.take_rate_dao_address,
        Addr::unchecked("take_rate_dao_address")
    );
    assert_eq!(config_res.is_take_rate_active, true);
    assert_eq!(config_res.take_rate, Decimal::from_str("0.5").unwrap());
}

#[test]
fn test_update_config_unsuccessfully_unauthorized() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("owner", &[]);
    mock_instantiation(deps.as_mut(), info.clone()).unwrap();

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: Config = from_json(&query_res).unwrap();
    assert_eq!(config_res.owner, Addr::unchecked("owner"));

    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        pool_router: None,
        fee_distributor: None,
        pool_factory: None,
        vault_factory: None,
        take_rate: Some(Decimal::one()),
        take_rate_dao_address: None,
        is_take_rate_active: None,
    };

    let err = execute(deps.as_mut(), mock_env(), info, msg);

    match err {
        Ok(_) => panic!("should return ContractError::InvalidTakeRate"),
        Err(ContractError::InvalidTakeRate {}) => (),
        _ => panic!("should return ContractError::InvalidTakeRate"),
    }

    let info = mock_info("unauthorized", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: Some("new_owner".to_string()),
        pool_router: None,
        fee_distributor: None,
        pool_factory: None,
        vault_factory: None,
        take_rate: None,
        take_rate_dao_address: None,
        is_take_rate_active: None,
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

    set_contract_version(
        &mut deps.storage,
        "notWW-fee_collector".to_string(),
        "1.0.0",
    )
    .unwrap();

    let res = migrate(deps.as_mut(), mock_env(), MigrateMsg {});
    // should not be able to migrate as the contract name is different Should be a StdError Contract name mismatch
    match res {
        Err(ContractError::Std { .. }) => {
            // Match the error message Contract name mismatch
            assert_eq!(
                res.unwrap_err().to_string(),
                "Generic error: Contract name mismatch".to_string()
            );
        }
        _ => panic!("should return ContractError::Std"),
    }
}
