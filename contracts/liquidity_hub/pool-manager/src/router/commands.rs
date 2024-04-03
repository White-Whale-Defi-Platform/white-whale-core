use cosmwasm_std::{
    coin, Addr, BankMsg, Decimal, DepsMut, MessageInfo, Response, StdError, Uint128,
};
use white_whale_std::{
    pool_manager::SwapOperation,
    pool_network::asset::{Asset, AssetInfo},
};

use crate::{state::MANAGER_CONFIG, swap::perform_swap::perform_swap, ContractError};

/// Checks that the output of each [`SwapOperation`] acts as the input of the next swap.
fn assert_operations(operations: Vec<SwapOperation>) -> Result<(), ContractError> {
    // check that the output of each swap is the input of the next swap
    let mut previous_output_info = operations
        .first()
        .ok_or(ContractError::NoSwapOperationsProvided {})?
        .get_input_asset_info()
        .clone();

    for operation in operations {
        if operation.get_input_asset_info() != &previous_output_info {
            return Err(ContractError::NonConsecutiveSwapOperations {
                previous_output: previous_output_info,
                next_input: operation.get_input_asset_info().clone(),
            });
        }

        previous_output_info = operation.get_target_asset_info();
    }

    Ok(())
}

pub fn execute_swap_operations(
    mut deps: DepsMut,
    info: MessageInfo,
    operations: Vec<SwapOperation>,
    minimum_receive: Option<Uint128>,
    to: Option<Addr>,
    max_spread: Option<Decimal>,
) -> Result<Response, ContractError> {
    let config = MANAGER_CONFIG.load(deps.storage)?;
    // check if the swap feature is enabled
    if !config.feature_toggle.swaps_enabled {
        return Err(ContractError::OperationDisabled("swap".to_string()));
    }

    // ensure that there was at least one operation
    // and retrieve the output token info
    let target_asset_info = operations
        .last()
        .ok_or(ContractError::NoSwapOperationsProvided {})?
        .get_target_asset_info();
    let target_denom = match &target_asset_info {
        AssetInfo::NativeToken { denom } => denom,
        _ => {
            return Err(ContractError::InvalidAsset {
                asset: target_asset_info.to_string(),
            })
        }
    };

    let offer_asset_info = operations
        .first()
        .ok_or(ContractError::NoSwapOperationsProvided {})?
        .get_input_asset_info();
    let offer_denom = match &offer_asset_info {
        AssetInfo::NativeToken { denom } => denom,
        _ => {
            return Err(ContractError::InvalidAsset {
                asset: offer_asset_info.to_string(),
            })
        }
    };
    let offer_asset = Asset {
        amount: info
            .funds
            .iter()
            .find(|token| &token.denom == offer_denom)
            .ok_or(ContractError::MissingNativeSwapFunds {
                denom: offer_denom.to_owned(),
            })?
            .amount,
        info: offer_asset_info.to_owned(),
    };

    assert_operations(operations.clone())?;

    // we return the output to the sender if no alternative recipient was specified.
    let to = to.unwrap_or(info.sender.clone());

    // perform each swap operation
    // we start off with the initial funds
    let mut previous_swap_output = offer_asset.clone();

    // stores messages for sending fees after the swaps
    let mut fee_messages = vec![];
    // stores swap attributes to add to tx info
    let mut swap_attributes = vec![];

    for operation in operations {
        match operation {
            SwapOperation::WhaleSwap {
                token_in_info,
                pool_identifier,
                ..
            } => match &token_in_info {
                AssetInfo::NativeToken { .. } => {
                    // inside assert_operations() we have already checked that
                    // the output of each swap is the input of the next swap.

                    let swap_result = perform_swap(
                        deps.branch(),
                        previous_swap_output.clone(),
                        pool_identifier,
                        None,
                        max_spread,
                    )?;
                    swap_attributes.push((
                        "swap",
                        format!(
                            "in={}, out={}, burn_fee={}, protocol_fee={}, swap_fee={}",
                            previous_swap_output,
                            swap_result.return_asset,
                            swap_result.burn_fee_asset,
                            swap_result.protocol_fee_asset,
                            swap_result.swap_fee_asset
                        ),
                    ));

                    // update the previous swap output
                    previous_swap_output = swap_result.return_asset;

                    // add the fee messages
                    if !swap_result.burn_fee_asset.amount.is_zero() {
                        fee_messages.push(swap_result.burn_fee_asset.into_burn_msg()?);
                    }
                    if !swap_result.protocol_fee_asset.amount.is_zero() {
                        fee_messages.push(
                            swap_result
                                .protocol_fee_asset
                                .into_msg(config.fee_collector_addr.clone())?,
                        );
                    }
                    if !swap_result.swap_fee_asset.amount.is_zero() {
                        fee_messages.push(
                            swap_result
                                .swap_fee_asset
                                .into_msg(config.fee_collector_addr.clone())?,
                        );
                    }
                }
                AssetInfo::Token { .. } => {
                    return Err(StdError::generic_err("cw20 token swaps are disabled"))?
                }
            },
        }
    }

    // Execute minimum amount assertion
    let receiver_balance = previous_swap_output.amount;
    if let Some(minimum_receive) = minimum_receive {
        if receiver_balance < minimum_receive {
            return Err(ContractError::MinimumReceiveAssertion {
                minimum_receive,
                swap_amount: receiver_balance,
            });
        }
    }

    // send output to recipient
    Ok(Response::new()
        .add_message(BankMsg::Send {
            to_address: to.to_string(),
            amount: vec![coin(receiver_balance.u128(), target_denom)],
        })
        .add_messages(fee_messages)
        .add_attributes(vec![
            ("action", "execute_swap_operations"),
            ("sender", info.sender.as_str()),
            ("receiver", to.as_str()),
            ("offer_info", &offer_asset.info.to_string()),
            ("offer_amount", &offer_asset.amount.to_string()),
            ("return_info", &target_asset_info.to_string()),
            ("return_amount", &receiver_balance.to_string()),
        ])
        .add_attributes(swap_attributes))
}
