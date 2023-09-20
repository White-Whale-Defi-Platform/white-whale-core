use cosmwasm_std::{
    to_binary, CosmosMsg, DepsMut, Env, MessageInfo, Order, OverflowError, OverflowOperation,
    Response, StdResult, Uint128, WasmMsg,
};

use white_whale::pool_network::asset::{Asset, AssetInfo};
use white_whale::pool_network::incentive::{Flow, FlowIdentifier};

use crate::error::ContractError;
use crate::execute::open_flow::DEFAULT_FLOW_DURATION;
use crate::helpers;
use crate::helpers::{get_flow_asset_amount_at_epoch, get_flow_end_epoch};
use crate::state::{EpochId, FlowId, FLOWS};

// If the end_epoch is not specified, the flow will be expanded by DEFAULT_FLOW_DURATION when
// the current epoch is within FLOW_EXPANSION_BUFFER epochs from the end_epoch.
const FLOW_EXPANSION_BUFFER: u64 = 5u64;

/// Expands a flow with the given id. Can be done by anyone.
pub fn expand_flow(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    flow_identifier: FlowIdentifier,
    end_epoch: Option<u64>,
    flow_asset: Asset,
) -> Result<Response, ContractError> {
    let flow: Option<((EpochId, FlowId), Flow)> = FLOWS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?
        .into_iter()
        .find(|(_, flow)| match &flow_identifier.clone() {
            FlowIdentifier::Id(id) => flow.flow_id == *id,
            FlowIdentifier::Label(label) => flow.flow_label.as_ref() == Some(label),
        });

    if let Some((_, mut flow)) = flow {
        // check if the flow has already ended
        let current_epoch = helpers::get_current_epoch(deps.as_ref())?;
        let expanded_end_epoch = get_flow_end_epoch(&flow);

        if current_epoch > expanded_end_epoch {
            return Err(ContractError::FlowAlreadyEnded {});
        }

        if flow.flow_asset.info != flow_asset.info {
            return Err(ContractError::FlowAssetNotSent {});
        }

        let mut messages: Vec<CosmosMsg> = vec![];

        // validate that the flow asset is sent to the contract
        match flow_asset.clone().info {
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

                // create the transfer message to send the flow asset to the contract
                messages.push(
                    WasmMsg::Execute {
                        contract_addr,
                        msg: to_binary(&cw20::Cw20ExecuteMsg::TransferFrom {
                            owner: info.sender.into_string(),
                            recipient: env.contract.address.into_string(),
                            amount: flow_asset.amount,
                        })?,
                        funds: vec![],
                    }
                    .into(),
                );
            }
            AssetInfo::NativeToken { denom } => {
                let paid_amount = cw_utils::must_pay(&info, &denom)?;
                if paid_amount != flow_asset.amount {
                    return Err(ContractError::MissingPositionDepositNative {
                        desired_amount: flow_asset.amount,
                        deposited_amount: paid_amount,
                    });
                }
                // all good, native tokens were sent
            }
        }

        // expand the flow only if the the epoch is within the expansion buffer.
        let expand_until =
            if expanded_end_epoch.saturating_sub(current_epoch) < FLOW_EXPANSION_BUFFER {
                expanded_end_epoch
                    .checked_add(DEFAULT_FLOW_DURATION)
                    .ok_or(ContractError::InvalidEndEpoch {})?
            } else {
                expanded_end_epoch
            };

        let end_epoch = end_epoch.unwrap_or(expand_until);

        // if the current end_epoch of this flow is greater than the new end_epoch, return error as
        // it wouldn't be expanding but contracting a flow.
        if expanded_end_epoch > end_epoch {
            return Err(ContractError::InvalidEndEpoch {});
        }

        // expand amount and end_epoch for the flow. The expansion happens from the next epoch.
        let next_epoch = current_epoch.checked_add(1u64).map_or_else(
            || {
                Err(OverflowError {
                    operation: OverflowOperation::Add,
                    operand1: current_epoch.to_string(),
                    operand2: 1u64.to_string(),
                })
            },
            Ok,
        )?;

        if let Some((existing_amount, expanded_end_epoch)) = flow.asset_history.get_mut(&next_epoch)
        {
            *existing_amount = existing_amount.checked_add(flow_asset.amount)?;
            *expanded_end_epoch = end_epoch;
        } else {
            // if there's no entry for the previous epoch, i.e. it is the first time the flow is expanded,
            // default to the original flow asset amount

            let expanded_amount = get_flow_asset_amount_at_epoch(&flow, current_epoch);
            //
            // let expanded_amount = if flow.asset_history.is_empty() {
            //     flow.flow_asset.amount
            // } else {
            //     flow.asset_history.range(..=current_epoch).rev().next()
            // };
            //
            // let expanded_amount = flow
            //     .asset_history
            //
            //     .get(&current_epoch)
            //     .unwrap_or(&flow.flow_asset.amount);

            flow.asset_history.insert(
                next_epoch,
                (expanded_amount.checked_add(flow_asset.amount)?, end_epoch),
            );
        }

        FLOWS.save(deps.storage, (flow.start_epoch, flow.flow_id), &flow)?;

        let total_flow_asset = flow
            .asset_history
            .values()
            .map(|&(expanded_amount, _)| expanded_amount)
            .sum::<Uint128>()
            .checked_add(flow.flow_asset.amount)?;

        Ok(Response::default().add_attributes(vec![
            ("action", "expand_flow".to_string()),
            ("flow_id", flow_identifier.to_string()),
            ("end_epoch", end_epoch.to_string()),
            ("expanding_flow_asset", flow_asset.to_string()),
            ("total_flow_asset", total_flow_asset.to_string()),
        ]))
    } else {
        Err(ContractError::NonExistentFlow {
            invalid_identifier: flow_identifier,
        })
    }
}
