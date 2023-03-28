use cosmwasm_std::{
    to_binary, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError, Uint128,
    WasmMsg,
};
use white_whale::pool_network::{
    asset::{Asset, AssetInfo},
    incentive::Curve,
};

use crate::{
    error::ContractError,
    state::{CONFIG, FLOWS, FLOW_COUNTER},
};

pub fn open_flow(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    start_timestamp: Option<u64>,
    end_timestamp: u64,
    curve: Curve,
    flow_asset: Asset,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let incentive_factory_addr = deps.api.addr_humanize(&config.factory_address)?;

    let incentive_factory_config: white_whale::pool_network::incentive_factory::GetConfigResponse =
        deps.querier.query_wasm_smart(
            incentive_factory_addr.into_string(),
            &white_whale::pool_network::incentive_factory::QueryMsg::GetConfig {},
        )?;

    let mut messages: Vec<CosmosMsg> = vec![];

    let flow_fee = incentive_factory_config.create_flow_fee;
    let fee_collector_addr = deps
        .api
        .addr_humanize(&incentive_factory_config.fee_collector_addr)?;
    match flow_fee.info.clone() {
        AssetInfo::NativeToken { denom } => {
            // fee should be included inside message info
            let paid_amount = info
                .funds
                .iter()
                .find(|token| token.denom == denom)
                .ok_or(ContractError::FlowFeeMissing)?
                .amount;

            if paid_amount < flow_fee.amount {
                return Err(ContractError::FlowFeeNotPaid {
                    paid_amount,
                    required_amount: flow_fee.amount,
                });
            } else if paid_amount > flow_fee.amount {
                // user sent more than required for the flow fee
                // refund them the difference
                messages.push(
                    BankMsg::Send {
                        to_address: info.sender.clone().into_string(),
                        amount: vec![Coin {
                            amount: paid_amount - flow_fee.amount,
                            denom: denom.clone(),
                        }],
                    }
                    .into(),
                );
            }

            // send fee to fee collector
            messages.push(
                BankMsg::Send {
                    to_address: fee_collector_addr.into_string(),
                    amount: vec![Coin {
                        amount: flow_fee.amount,
                        denom,
                    }],
                }
                .into(),
            );
        }
        AssetInfo::Token { contract_addr } => {
            // we should have been given permissions through allowances
            let allowance: cw20::AllowanceResponse = deps.querier.query_wasm_smart(
                contract_addr.clone(),
                &cw20::Cw20QueryMsg::Allowance {
                    owner: info.sender.clone().into_string(),
                    spender: env.contract.address.clone().into_string(),
                },
            )?;

            if allowance.allowance < flow_fee.amount {
                return Err(ContractError::FlowFeeNotPaid {
                    paid_amount: allowance.allowance,
                    required_amount: flow_fee.amount,
                });
            }

            // send fee to fee collector
            messages.push(
                WasmMsg::Execute {
                    contract_addr,
                    msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                        recipient: fee_collector_addr.into_string(),
                        amount: flow_fee.amount,
                    })?,
                    funds: vec![],
                }
                .into(),
            );
        }
    }

    // verify that not too many flows have been made for this LP token
    let flows = u64::try_from(FLOWS.load(deps.storage)?.len())
        .map_err(|_| StdError::generic_err("Failed to parse flow count"))?;
    if flows >= incentive_factory_config.max_concurrent_flows {
        return Err(ContractError::TooManyFlows {
            maximum: incentive_factory_config.max_concurrent_flows,
        });
    }

    // transfer the `flow_asset` over to us if it was a cw20 token
    // otherwise, verify the user sent the claimed amount in `info.funds`
    match flow_asset.info.clone() {
        AssetInfo::NativeToken { denom } => {
            info.funds
                .iter()
                .find(|sent| sent.denom == denom && sent.amount == flow_asset.amount)
                .ok_or(ContractError::FlowAssetNotSent)?;
        }
        AssetInfo::Token { contract_addr } => {
            let allowance: cw20::AllowanceResponse = deps.querier.query_wasm_smart(
                contract_addr.clone(),
                &cw20::Cw20QueryMsg::Allowance {
                    owner: info.sender.clone().into_string(),
                    spender: env.contract.address.clone().into_string(),
                },
            )?;

            if allowance.allowance < flow_asset.amount {
                return Err(ContractError::FlowAssetNotSent);
            }

            // create the transfer message to us
            messages.push(
                WasmMsg::Execute {
                    contract_addr,
                    msg: to_binary(&cw20::Cw20ExecuteMsg::TransferFrom {
                        owner: info.sender.clone().into_string(),
                        recipient: env.contract.address.into_string(),
                        amount: flow_asset.amount,
                    })?,
                    funds: vec![],
                }
                .into(),
            );
        }
    }

    // ensure the flow is set for a expire date in the future
    if env.block.time.seconds() > end_timestamp {
        return Err(ContractError::FlowExpirationInPast);
    }

    // ensure that start date is set within buffer
    let start_timestamp = start_timestamp.unwrap_or(env.block.time.seconds());
    if start_timestamp
        > env.block.time.seconds() + incentive_factory_config.max_flow_start_time_buffer
    {
        return Err(ContractError::FlowStartTooFar);
    }

    // add the flow
    let flow_id =
        FLOW_COUNTER.update::<_, StdError>(deps.storage, |current_id| Ok(current_id + 1))?;
    FLOWS.update::<_, StdError>(deps.storage, |mut flows| {
        flows.push(white_whale::pool_network::incentive::Flow {
            flow_creator: deps.api.addr_canonicalize(&info.sender.into_string())?,
            flow_id,
            curve,
            flow_asset,
            claimed_amount: Uint128::zero(),
            start_timestamp,
            end_timestamp,
        });

        Ok(flows)
    })?;

    Ok(Response::new().add_messages(messages))
}
