use std::cmp::Ordering;
use std::collections::HashMap;

use cosmwasm_std::{
    to_json_binary, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Order, Response, StdError,
    StdResult, Uint128, WasmMsg,
};

use white_whale_std::pool_network::incentive::Flow;
use white_whale_std::pool_network::{
    asset::{Asset, AssetInfo},
    incentive::Curve,
};

use crate::{
    error::ContractError,
    helpers,
    state::{CONFIG, FLOWS, FLOW_COUNTER},
};

const MIN_FLOW_AMOUNT: Uint128 = Uint128::new(1_000u128);
pub const DEFAULT_FLOW_DURATION: u64 = 14u64;

/// Opens a flow to incentivize liquidity providers
#[allow(clippy::too_many_arguments)]
pub fn open_flow(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    start_epoch: Option<u64>,
    end_epoch: Option<u64>,
    curve: Option<Curve>,
    mut flow_asset: Asset,
    flow_label: Option<String>,
) -> Result<Response, ContractError> {
    // check the user is not trying to create an empty flow
    if flow_asset.amount < MIN_FLOW_AMOUNT {
        return Err(ContractError::EmptyFlow {
            min: MIN_FLOW_AMOUNT,
        });
    }

    let config = CONFIG.load(deps.storage)?;

    let incentive_factory_config: white_whale_std::pool_network::incentive_factory::ConfigResponse =
        deps.querier.query_wasm_smart(
            config.factory_address.into_string(),
            &white_whale_std::pool_network::incentive_factory::QueryMsg::Config {},
        )?;

    let mut messages: Vec<CosmosMsg> = vec![];

    let flow_fee = incentive_factory_config.create_flow_fee;
    // check the fee to create a flow is being paid
    match flow_fee.info.clone() {
        AssetInfo::NativeToken {
            denom: flow_fee_denom,
        } => {
            // fee should be included inside message info
            let paid_amount = info
                .funds
                .iter()
                .find(|token| token.denom == flow_fee_denom)
                .ok_or(ContractError::FlowFeeMissing)?
                .amount;

            // check if the user intends to open a flow with the same asset used to pay for the flow_fee
            match flow_asset.info.clone() {
                AssetInfo::Token { .. } => {}
                AssetInfo::NativeToken {
                    denom: flow_asset_denom,
                } => {
                    // if so, subtract the flow_fee from the flow_asset amount
                    if flow_fee_denom == flow_asset_denom {
                        flow_asset.amount = flow_asset.amount.saturating_sub(flow_fee.amount);

                        if flow_asset.amount < MIN_FLOW_AMOUNT {
                            return Err(ContractError::EmptyFlowAfterFee {
                                min: MIN_FLOW_AMOUNT,
                            });
                        }
                    }
                }
            }

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
                    // if the user is paying more than the flow_fee and is not trying to open a
                    // flow with the same asset as the flow_fee, refund the difference
                    match flow_asset.info.clone() {
                        AssetInfo::Token { .. } => {}
                        AssetInfo::NativeToken {
                            denom: flow_asset_denom,
                        } => {
                            if flow_fee_denom != flow_asset_denom {
                                messages.push(
                                    BankMsg::Send {
                                        to_address: info.sender.clone().into_string(),
                                        amount: vec![Coin {
                                            amount: paid_amount - flow_fee.amount,
                                            denom: flow_fee_denom.clone(),
                                        }],
                                    }
                                    .into(),
                                );
                            }
                        }
                    }
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
                        denom: flow_fee_denom,
                    }],
                }
                .into(),
            );
        }
        AssetInfo::Token {
            contract_addr: flow_fee_contract_addr,
        } => {
            // we should have been given permissions through allowances
            let flow_fee_allowance: cw20::AllowanceResponse = deps.querier.query_wasm_smart(
                flow_fee_contract_addr.clone(),
                &cw20::Cw20QueryMsg::Allowance {
                    owner: info.sender.clone().into_string(),
                    spender: env.contract.address.clone().into_string(),
                },
            )?;

            match flow_asset.info.clone() {
                AssetInfo::Token {
                    contract_addr: flow_asset_contract_addr,
                } => {
                    if flow_asset_contract_addr == flow_fee_contract_addr {
                        if flow_fee_allowance.allowance
                            < flow_fee.amount.checked_add(MIN_FLOW_AMOUNT)?
                        {
                            return Err(ContractError::EmptyFlowAfterFee {
                                min: MIN_FLOW_AMOUNT,
                            });
                        }
                    } else if flow_fee_allowance.allowance < flow_fee.amount {
                        return Err(ContractError::FlowFeeNotPaid {
                            paid_amount: flow_fee_allowance.allowance,
                            required_amount: flow_fee.amount,
                        });
                    }
                }
                AssetInfo::NativeToken { .. } => {
                    if flow_fee_allowance.allowance < flow_fee.amount {
                        return Err(ContractError::FlowFeeNotPaid {
                            paid_amount: flow_fee_allowance.allowance,
                            required_amount: flow_fee.amount,
                        });
                    }
                }
            }

            // send fee to fee collector
            messages.push(
                WasmMsg::Execute {
                    contract_addr: flow_fee_contract_addr,
                    msg: to_json_binary(&cw20::Cw20ExecuteMsg::TransferFrom {
                        owner: info.sender.clone().into_string(),
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
    let flows = u64::try_from(
        FLOWS
            .range(deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?
            .into_iter()
            .len(),
    )
    .map_err(|_| StdError::generic_err("Failed to parse flow count"))?;
    if flows >= incentive_factory_config.max_concurrent_flows {
        return Err(ContractError::TooManyFlows {
            maximum: incentive_factory_config.max_concurrent_flows,
        });
    }

    // transfer the `flow_asset` over to us if it was a cw20 token
    // otherwise, verify the user sent the claimed amount in `info.funds`
    match flow_asset.info.clone() {
        AssetInfo::NativeToken {
            denom: flow_asset_denom,
        } => {
            match flow_fee.info {
                AssetInfo::Token { .. } => {
                    info.funds
                        .iter()
                        .find(|sent| {
                            sent.denom == flow_asset_denom && sent.amount == flow_asset.amount
                        })
                        .ok_or(ContractError::FlowAssetNotSent)?;
                }
                AssetInfo::NativeToken {
                    denom: flow_fee_denom,
                } => {
                    if flow_fee_denom != flow_asset_denom {
                        info.funds
                            .iter()
                            .find(|sent| {
                                sent.denom == flow_asset_denom && sent.amount == flow_asset.amount
                            })
                            .ok_or(ContractError::FlowAssetNotSent)?;
                    }
                    // no need to verify the case where flow_fee_denom == flow_asset_denom since
                    // it is done before when we check the fee_flow denom is the same as the flow_asset_denom
                }
            }
        }
        AssetInfo::Token {
            contract_addr: flow_asset_contract_addr,
        } => {
            let allowance: cw20::AllowanceResponse = deps.querier.query_wasm_smart(
                flow_asset_contract_addr.clone(),
                &cw20::Cw20QueryMsg::Allowance {
                    owner: info.sender.clone().into_string(),
                    spender: env.contract.address.clone().into_string(),
                },
            )?;

            match flow_fee.info.clone() {
                AssetInfo::Token {
                    contract_addr: flow_fee_contract_addr,
                } => {
                    if flow_fee_contract_addr != flow_asset_contract_addr {
                        if allowance.allowance < flow_fee.amount {
                            return Err(ContractError::FlowAssetNotSent);
                        }

                        // create the transfer message to us
                        messages.push(
                            WasmMsg::Execute {
                                contract_addr: flow_asset_contract_addr,
                                msg: to_json_binary(&cw20::Cw20ExecuteMsg::TransferFrom {
                                    owner: info.sender.clone().into_string(),
                                    recipient: env.contract.address.into_string(),
                                    amount: flow_asset.amount,
                                })?,
                                funds: vec![],
                            }
                            .into(),
                        );
                    } else {
                        // if the flow_fee contract is the same as the flow_asset contract,
                        // then we need to check the allowance is enough for both the flow_fee and MIN_FLOW_AMOUNT
                        if allowance.allowance < flow_fee.amount.checked_add(MIN_FLOW_AMOUNT)? {
                            return Err(ContractError::EmptyFlowAfterFee {
                                min: MIN_FLOW_AMOUNT,
                            });
                        }
                        // no need to verify the allowance can cover for both the flow_fee and MIN_FLOW_AMOUNT
                        // since it was already done above

                        // subtract the flow_fee from the flow_asset amount
                        flow_asset.amount = flow_asset.amount.saturating_sub(flow_fee.amount);

                        // if the allowance covers both for the flow_fee and MIN_FLOW_AMOUNT,
                        // send the rest to us, i.e. the flow_asset.amount - the fee_flow.amount
                        // the fee_flow.amount is being sent to the fee_collector_addr above
                        messages.push(
                            WasmMsg::Execute {
                                contract_addr: flow_asset_contract_addr,
                                msg: to_json_binary(&cw20::Cw20ExecuteMsg::TransferFrom {
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
                AssetInfo::NativeToken { .. } => {
                    if allowance.allowance < flow_asset.amount {
                        return Err(ContractError::FlowAssetNotSent);
                    }

                    messages.push(
                        WasmMsg::Execute {
                            contract_addr: flow_asset_contract_addr,
                            msg: to_json_binary(&cw20::Cw20ExecuteMsg::TransferFrom {
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
        }
    }

    let current_epoch = helpers::get_current_epoch(deps.as_ref())?;
    let end_epoch = end_epoch.unwrap_or(
        current_epoch
            .checked_add(DEFAULT_FLOW_DURATION)
            .ok_or(ContractError::InvalidEndEpoch {})?,
    );

    // ensure the flow is set for a expire date in the future
    if current_epoch > end_epoch {
        return Err(ContractError::FlowExpirationInPast);
    }

    let start_epoch = start_epoch.unwrap_or(current_epoch);

    // ensure that start date is before end date
    if start_epoch > end_epoch {
        return Err(ContractError::FlowStartTimeAfterEndTime);
    }

    // ensure that start date is set within buffer
    if start_epoch > current_epoch + incentive_factory_config.max_flow_epoch_buffer {
        return Err(ContractError::FlowStartTooFar);
    }

    //let emissions_per_epoch = flow_asset.amount.checked_div_euclid(Uint128::from(flow_duration_in_epochs))?;
    let emissions_per_epoch = Uint128::zero();

    // add the flow
    let flow_id =
        FLOW_COUNTER.update::<_, StdError>(deps.storage, |current_id| Ok(current_id + 1u64))?;

    let curve = curve.unwrap_or(Curve::Linear);

    FLOWS.save(
        deps.storage,
        (start_epoch, flow_id),
        &Flow {
            flow_creator: info.sender.clone(),
            flow_id,
            flow_label: flow_label.clone(),
            curve: curve.clone(),
            flow_asset: flow_asset.clone(),
            claimed_amount: Uint128::zero(),
            start_epoch,
            end_epoch,
            emitted_tokens: HashMap::new(),
            asset_history: Default::default(),
        },
    )?;

    let mut attributes = vec![("action", "open_flow".to_string())];

    if let Some(flow_label) = flow_label {
        attributes.push(("flow_label", flow_label));
    }

    attributes.extend(vec![
        ("flow_id", flow_id.to_string()),
        ("flow_creator", info.sender.into_string()),
        ("flow_asset", flow_asset.info.to_string()),
        ("flow_asset_amount", flow_asset.amount.to_string()),
        ("start_epoch", start_epoch.to_string()),
        ("end_epoch", end_epoch.to_string()),
        ("emissions_per_epoch", emissions_per_epoch.to_string()),
        ("curve", curve.to_string()),
    ]);

    Ok(Response::default()
        .add_attributes(attributes)
        .add_messages(messages))
}
