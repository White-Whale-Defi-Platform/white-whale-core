use cosmwasm_std::{DepsMut, Response};
use white_whale::pool_network::asset::Asset;

use crate::{error::ContractError, state::CONFIG};

pub fn update_config(
    deps: DepsMut,
    fee_collector_addr: Option<String>,
    create_flow_fee: Option<Asset>,
    max_concurrent_flows: Option<u64>,
    incentive_contract_id: Option<u64>,
    max_flow_start_time_buffer: Option<u64>,
) -> Result<Response, ContractError> {
    CONFIG.update::<_, ContractError>(deps.storage, |mut config| {
        if let Some(fee_collector_addr) = fee_collector_addr {
            config.fee_collector_addr = deps.api.addr_canonicalize(&fee_collector_addr)?;
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

        if let Some(incentive_contract_id) = incentive_contract_id {
            config.incentive_code_id = incentive_contract_id;
        }

        if let Some(max_flow_start_time_buffer) = max_flow_start_time_buffer {
            config.max_flow_start_time_buffer = max_flow_start_time_buffer;
        }

        Ok(config)
    })?;

    Ok(Response::new())
}
