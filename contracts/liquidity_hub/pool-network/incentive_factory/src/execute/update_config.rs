use cosmwasm_std::{DepsMut, Response};
use white_whale_std::pool_network::asset::Asset;

use crate::{error::ContractError, state::CONFIG};

#[allow(clippy::too_many_arguments)]
pub fn update_config(
    deps: DepsMut,
    owner: Option<String>,
    fee_collector_addr: Option<String>,
    fee_distributor_addr: Option<String>,
    create_flow_fee: Option<Asset>,
    max_concurrent_flows: Option<u64>,
    incentive_code_id: Option<u64>,
    max_flow_start_time_buffer: Option<u64>,
    min_unbonding_duration: Option<u64>,
    max_unbonding_duration: Option<u64>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if let Some(owner) = owner {
        config.owner = deps.api.addr_validate(&owner)?;
    }

    if let Some(fee_collector_addr) = fee_collector_addr {
        config.fee_collector_addr = deps.api.addr_validate(&fee_collector_addr)?;
    }

    if let Some(fee_distributor_addr) = fee_distributor_addr {
        config.fee_distributor_addr = deps.api.addr_validate(&fee_distributor_addr)?;
    }

    if let Some(create_flow_fee) = create_flow_fee {
        config.create_flow_fee = create_flow_fee;
    }

    if let Some(max_concurrent_flows) = max_concurrent_flows {
        if max_concurrent_flows == 0 {
            return Err(ContractError::UnspecifiedConcurrentFlows);
        }

        config.max_concurrent_flows = max_concurrent_flows;
    }

    if let Some(incentive_code_id) = incentive_code_id {
        config.incentive_code_id = incentive_code_id;
    }

    if let Some(max_flow_start_time_buffer) = max_flow_start_time_buffer {
        config.max_flow_epoch_buffer = max_flow_start_time_buffer;
    }

    if let Some(max_unbonding_duration) = max_unbonding_duration {
        if max_unbonding_duration < config.min_unbonding_duration {
            return Err(ContractError::InvalidUnbondingRange {
                min: config.min_unbonding_duration,
                max: max_unbonding_duration,
            });
        }

        config.max_unbonding_duration = max_unbonding_duration;
    }

    if let Some(min_unbonding_duration) = min_unbonding_duration {
        if config.max_unbonding_duration < min_unbonding_duration {
            return Err(ContractError::InvalidUnbondingRange {
                min: min_unbonding_duration,
                max: config.max_unbonding_duration,
            });
        }

        config.min_unbonding_duration = min_unbonding_duration;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default().add_attributes(vec![
        ("action", "update_config".to_string()),
        ("owner", config.owner.to_string()),
        ("fee_collector_addr", config.fee_collector_addr.to_string()),
        (
            "fee_distributor_addr",
            config.fee_distributor_addr.to_string(),
        ),
        ("create_flow_fee", config.create_flow_fee.to_string()),
        (
            "max_concurrent_flows",
            config.max_concurrent_flows.to_string(),
        ),
        ("incentive_code_id", config.incentive_code_id.to_string()),
        (
            "max_flow_start_time_buffer",
            config.max_flow_epoch_buffer.to_string(),
        ),
        (
            "min_unbonding_duration",
            config.min_unbonding_duration.to_string(),
        ),
        (
            "max_unbonding_duration",
            config.max_unbonding_duration.to_string(),
        ),
    ]))
}

#[cfg(test)]
mod tests {
    // create test to check the update_config function works properly

    use crate::contract::{execute, instantiate, query};
    use crate::error::ContractError;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_json, Addr, Uint128};
    use white_whale_std::pool_network::asset::{Asset, AssetInfo};
    use white_whale_std::pool_network::incentive_factory::ExecuteMsg::UpdateConfig;
    use white_whale_std::pool_network::incentive_factory::{Config, InstantiateMsg, QueryMsg};

    #[test]
    fn update_config_successfully() {
        let mut deps = mock_dependencies();
        let info = mock_info("owner", &[]);

        let msg = InstantiateMsg {
            fee_collector_addr: "fee_collector_addr".to_string(),
            fee_distributor_addr: "fee_distributor_addr".to_string(),
            create_flow_fee: Asset {
                info: AssetInfo::NativeToken {
                    denom: "native-fee-token".to_string(),
                },
                amount: Uint128::one(),
            },
            max_concurrent_flows: 1u64,
            incentive_code_id: 123,
            max_flow_epoch_buffer: 3600u64,
            min_unbonding_duration: 86400u64,
            max_unbonding_duration: 259200u64,
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let config: Config =
            from_json(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();

        assert_eq!(
            config,
            Config {
                owner: Addr::unchecked("owner"),
                fee_collector_addr: Addr::unchecked("fee_collector_addr"),
                fee_distributor_addr: Addr::unchecked("fee_distributor_addr"),
                create_flow_fee: Asset {
                    info: AssetInfo::NativeToken {
                        denom: "native-fee-token".to_string()
                    },
                    amount: Uint128::one()
                },
                max_concurrent_flows: 1u64,
                incentive_code_id: 123,
                max_flow_epoch_buffer: 3600u64,
                min_unbonding_duration: 86400u64,
                max_unbonding_duration: 259200u64,
            }
        );

        let msg = UpdateConfig {
            owner: Some("new_owner".to_string()),
            fee_collector_addr: Some("new_fee_collector_addr".to_string()),
            fee_distributor_addr: Some("new_fee_distributor_addr".to_string()),
            create_flow_fee: Some(Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(1000u128),
            }),
            max_concurrent_flows: Some(10u64),
            incentive_code_id: Some(456u64),
            max_flow_start_time_buffer: Some(60u64),
            min_unbonding_duration: Some(1000u64),
            max_unbonding_duration: Some(86400u64),
        };

        let info = mock_info("owner", &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let config: Config =
            from_json(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();

        assert_eq!(
            config,
            Config {
                owner: Addr::unchecked("new_owner"),
                fee_collector_addr: Addr::unchecked("new_fee_collector_addr"),
                fee_distributor_addr: Addr::unchecked("new_fee_distributor_addr"),
                create_flow_fee: Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string()
                    },
                    amount: Uint128::new(1000u128)
                },
                max_concurrent_flows: 10u64,
                incentive_code_id: 456u64,
                max_flow_epoch_buffer: 60u64,
                min_unbonding_duration: 1000u64,
                max_unbonding_duration: 86400u64,
            }
        );
    }

    #[test]
    fn update_config_unsuccessfully() {
        let mut deps = mock_dependencies();
        let info = mock_info("owner", &[]);

        let msg = InstantiateMsg {
            fee_collector_addr: "fee_collector_addr".to_string(),
            fee_distributor_addr: "fee_distributor_addr".to_string(),
            create_flow_fee: Asset {
                info: AssetInfo::NativeToken {
                    denom: "native-fee-token".to_string(),
                },
                amount: Uint128::one(),
            },
            max_concurrent_flows: 1u64,
            incentive_code_id: 123,
            max_flow_epoch_buffer: 3600u64,
            min_unbonding_duration: 86400u64,
            max_unbonding_duration: 259200u64,
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("unauthorized", &[]);
        let msg = UpdateConfig {
            owner: None,
            fee_collector_addr: None,
            fee_distributor_addr: None,
            create_flow_fee: None,
            max_concurrent_flows: Some(0u64),
            incentive_code_id: None,
            max_flow_start_time_buffer: None,
            min_unbonding_duration: None,
            max_unbonding_duration: None,
        };

        let err = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap_err();
        match err {
            ContractError::Unauthorized => {}
            _ => panic!("should return ContractError::Unauthorized"),
        }

        let info = mock_info("owner", &[]);
        let err = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();

        match err {
            ContractError::UnspecifiedConcurrentFlows => {}
            _ => panic!("should return ContractError::UnspecifiedConcurrentFlows"),
        }

        let msg = UpdateConfig {
            owner: None,
            fee_collector_addr: None,
            fee_distributor_addr: None,
            create_flow_fee: None,
            max_concurrent_flows: None,
            incentive_code_id: None,
            max_flow_start_time_buffer: None,
            min_unbonding_duration: Some(300000u64),
            max_unbonding_duration: None,
        };

        let err = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap_err();
        match err {
            ContractError::InvalidUnbondingRange { .. } => {}
            _ => panic!("should return ContractError::InvalidUnbondingRange"),
        }

        let msg = UpdateConfig {
            owner: None,
            fee_collector_addr: None,
            fee_distributor_addr: None,
            create_flow_fee: None,
            max_concurrent_flows: None,
            incentive_code_id: None,
            max_flow_start_time_buffer: None,
            min_unbonding_duration: None,
            max_unbonding_duration: Some(1000u64),
        };

        let err = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap_err();
        match err {
            ContractError::InvalidUnbondingRange { .. } => {}
            _ => panic!("should return ContractError::InvalidUnbondingRange"),
        }
    }
}
