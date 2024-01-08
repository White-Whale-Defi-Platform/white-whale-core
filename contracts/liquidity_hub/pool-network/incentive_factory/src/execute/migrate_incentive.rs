use classic_bindings::TerraQuery;
use cosmwasm_std::{to_binary, Addr, CosmosMsg, DepsMut, Order, Response, StdResult, WasmMsg};

use crate::error::ContractError;
use crate::state::INCENTIVE_MAPPINGS;

pub fn migrate_incentives(
    deps: DepsMut<TerraQuery>,
    incentive_address: Option<String>,
    code_id: u64,
) -> Result<Response, ContractError> {
    // migrate only the provided incentive address, otherwise migrate all incentives
    let mut res = Response::new().add_attributes(vec![
        ("method", "migrate_incentives".to_string()),
        ("code_id", code_id.to_string()),
    ]);
    if let Some(incentive_address) = incentive_address {
        Ok(res
            .add_attribute("incentive", incentive_address.clone())
            .add_message(migrate_incentive_msg(
                deps.api.addr_validate(incentive_address.as_str())?,
                code_id,
            )?))
    } else {
        let incentives = INCENTIVE_MAPPINGS
            .range(deps.storage, None, None, Order::Ascending)
            .take(30usize)
            .map(|item| {
                let (_, incentive_address) = item?;
                Ok(incentive_address)
            })
            .collect::<StdResult<Vec<Addr>>>()?;

        for incentive in incentives {
            res = res
                .add_attribute("incentive", &incentive.clone().to_string())
                .add_message(migrate_incentive_msg(
                    deps.api.addr_validate(incentive.into_string().as_str())?,
                    code_id,
                )?)
        }

        Ok(res)
    }
}

/// Creates a migrate incentive message given a incentive address and code id
fn migrate_incentive_msg(incentive_address: Addr, new_code_id: u64) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Wasm(WasmMsg::Migrate {
        contract_addr: incentive_address.to_string(),
        new_code_id,
        msg: to_binary(&white_whale::pool_network::incentive::MigrateMsg {})?,
    }))
}

#[cfg(test)]
mod tests {
    // create test to check the update_config function works properly

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{attr, Addr, Uint128};

    use white_whale::pool_network::asset::{Asset, AssetInfo};
    use white_whale::pool_network::incentive_factory::ExecuteMsg::MigrateIncentives;
    use white_whale::pool_network::incentive_factory::InstantiateMsg;

    use crate::contract::{execute, instantiate};
    use crate::state::INCENTIVE_MAPPINGS;

    #[test]
    fn migrate_single_incentive() {
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

        for i in 0..3 {
            INCENTIVE_MAPPINGS
                .save(
                    deps.as_mut().storage,
                    ("token".to_string() + i.to_string().as_str()).as_bytes(),
                    &Addr::unchecked("incentive".to_string() + i.to_string().as_str()),
                )
                .unwrap();
        }

        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("owner", &[]),
            MigrateIncentives {
                incentive_address: Some("incentive0".to_string()),
                code_id: 456,
            },
        )
        .unwrap();

        let expected_attributes = vec![
            attr("method", "migrate_incentives"),
            attr("code_id", "456"),
            attr("incentive", "incentive0"),
        ];
        assert_eq!(res.attributes, expected_attributes);
        assert_eq!(res.messages.len(), 1usize);
    }
    #[test]
    fn migrate_multiple_incentives() {
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

        for i in 0..3 {
            INCENTIVE_MAPPINGS
                .save(
                    deps.as_mut().storage,
                    ("token".to_string() + i.to_string().as_str()).as_bytes(),
                    &Addr::unchecked("incentive".to_string() + i.to_string().as_str()),
                )
                .unwrap();
        }

        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("owner", &[]),
            MigrateIncentives {
                incentive_address: None,
                code_id: 456,
            },
        )
        .unwrap();

        let expected_attributes = vec![
            attr("method", "migrate_incentives"),
            attr("code_id", "456"),
            attr("incentive", "incentive0"),
            attr("incentive", "incentive1"),
            attr("incentive", "incentive2"),
        ];
        assert_eq!(res.attributes, expected_attributes);
        assert_eq!(res.messages.len(), 3usize);
    }
}
