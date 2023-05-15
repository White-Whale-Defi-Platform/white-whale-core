use std::cmp::Ordering;

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

    let incentive_factory_config: white_whale::pool_network::incentive_factory::ConfigResponse =
        deps.querier.query_wasm_smart(
            config.factory_address.into_string(),
            &white_whale::pool_network::incentive_factory::QueryMsg::Config {},
        )?;

    let mut messages: Vec<CosmosMsg> = vec![];

    let flow_fee = incentive_factory_config.create_flow_fee;
    match flow_fee.info.clone() {
        AssetInfo::NativeToken { denom } => {
            // fee should be included inside message info
            let paid_amount = info
                .funds
                .iter()
                .find(|token| token.denom == denom)
                .ok_or(ContractError::FlowFeeMissing)?
                .amount;

            match paid_amount.cmp(&flow_fee.amount) {
                Ordering::Equal => (), // do nothing if user paid correct amount,
                Ordering::Less => {
                    // user underpaid
                    return Err(ContractError::FlowFeeNotPaid {
                        paid_amount,
                        required_amount: flow_fee.amount,
                    });
                }
                Ordering::Greater => {
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
            }

            // send fee to fee collector
            messages.push(
                BankMsg::Send {
                    to_address: incentive_factory_config
                        .fee_collector_addr
                        .clone()
                        .into_string(),
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
                        recipient: incentive_factory_config.fee_collector_addr.into_string(),
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
            flow_creator: info.sender.clone(),
            flow_id: flow_id.clone(),
            curve: curve.clone(),
            flow_asset: flow_asset.clone(),
            claimed_amount: Uint128::zero(),
            start_timestamp: start_timestamp.clone(),
            end_timestamp: end_timestamp.clone(),
        });

        Ok(flows)
    })?;

    Ok(Response::default()
        .add_attributes(vec![
            ("action", "open_flow".to_string()),
            ("flow_id", flow_id.to_string()),
            ("flow_creator", info.sender.into_string()),
            ("flow_asset", flow_asset.info.to_string()),
            ("flow_asset_amount", flow_asset.amount.to_string()),
            ("start_timestamp", start_timestamp.to_string()),
            ("end_timestamp", end_timestamp.to_string()),
            ("curve", curve.to_string()),
        ])
        .add_messages(messages))
}
