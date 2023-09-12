use cosmwasm_std::{
    to_binary, CosmosMsg, DepsMut, Env, MessageInfo, Order, OverflowError, OverflowOperation,
    Response, StdResult, Uint128, WasmMsg,
};

use white_whale::pool_network::asset::{Asset, AssetInfo};
use white_whale::pool_network::incentive::Flow;

use crate::error::ContractError;
use crate::helpers;
use crate::state::{EpochId, FlowId, FLOWS};

/// Expands a flow with the given id.
pub fn expand_flow(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    flow_id: u64,
    end_epoch: u64,
    flow_asset: Asset,
) -> Result<Response, ContractError> {
    let flow: Option<((EpochId, FlowId), Flow)> = FLOWS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?
        .into_iter()
        .find(|(_, flow)| flow.flow_id == flow_id);

    if let Some((_, mut flow)) = flow {
        // validate that user is allowed to expand the flow
        if flow.flow_creator != info.sender {
            return Err(ContractError::Unauthorized {});
        }

        // check if the flow has already ended
        let current_epoch = helpers::get_current_epoch(deps.as_ref())?;
        if current_epoch > flow.end_epoch {
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

        // if the current end_epoch of this flow is greater than the new end_epoch, return error as
        // it wouldn't be expanding but contracting a flow.
        if flow.end_epoch > end_epoch {
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

        if let Some(existing_amount) = flow.asset_history.get_mut(&next_epoch) {
            *existing_amount = existing_amount.checked_add(flow_asset.amount)?;
        } else {
            flow.asset_history.insert(
                next_epoch,
                flow.flow_asset.amount.checked_add(flow_asset.amount)?,
            );
        }

        flow.end_epoch = end_epoch;
        FLOWS.save(deps.storage, (flow.start_epoch, flow.flow_id), &flow)?;

        let total_flow_asset = flow
            .asset_history
            .values()
            .copied()
            .sum::<Uint128>()
            .checked_add(flow.flow_asset.amount)?;

        Ok(Response::default().add_attributes(vec![
            ("action", "expand_flow".to_string()),
            ("flow_id", flow_id.to_string()),
            ("end_epoch", end_epoch.to_string()),
            ("expanding_flow_asset", flow_asset.to_string()),
            ("total_flow_asset", total_flow_asset.to_string()),
        ]))
    } else {
        Err(ContractError::NonExistentFlow {
            invalid_id: flow_id,
        })
    }
}
