use std::collections::HashMap;

use cosmwasm_std::{
    coin, Addr, BankMsg, Decimal, DepsMut, MessageInfo, Response, StdError, Uint128,
};
use white_whale_std::{
    pool_manager::SwapOperation,
    pool_network::asset::{Asset, AssetInfo},
};

use crate::{state::MANAGER_CONFIG, swap::perform_swap::perform_swap, ContractError};

/// Checks that an arbitrary amount of [`SwapOperation`]s will not result in
/// multiple output tokens.
fn assert_operations(operations: &[SwapOperation]) -> Result<(), ContractError> {
    let mut ask_asset_map: HashMap<String, bool> = HashMap::new();
    for operation in operations.iter() {
        let (offer_asset, ask_asset, _pool_identifier) = match operation {
            SwapOperation::WhaleSwap {
                token_in_info: offer_asset_info,
                token_out_info: ask_asset_info,
                pool_identifier,
            } => (
                offer_asset_info.clone(),
                ask_asset_info.clone(),
                pool_identifier.clone(),
            ),
        };

        ask_asset_map.remove(&offer_asset.to_string());
        ask_asset_map.insert(ask_asset.to_string(), true);
    }

    if ask_asset_map.keys().len() != 1 {
        return Err(ContractError::MultipleOutputToken {});
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

    assert_operations(&operations)?;

    // we return the output to the sender if no alternative recipient was specified.
    let to = to.unwrap_or(info.sender.clone());

    // perform each swap operation
    // we start off with the initial funds
    let mut previous_swap_output = offer_asset.clone();

    // stores messages for sending fees after the swaps
    let mut fee_messages = vec![];

    for operation in operations {
        match operation {
            SwapOperation::WhaleSwap {
                token_in_info,
                pool_identifier,
                ..
            } => match &token_in_info {
                AssetInfo::NativeToken { .. } => {
                    let swap_result = perform_swap(
                        deps.branch(),
                        previous_swap_output,
                        pool_identifier,
                        None,
                        max_spread,
                    )?;

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
            ("sender", &info.sender.as_str()),
            ("receiver", to.as_str()),
            ("offer_info", &offer_asset.info.to_string()),
            ("offer_amount", &offer_asset.amount.to_string()),
            ("return_info", &target_asset_info.to_string()),
            ("return_amount", &receiver_balance.to_string()),
        ]))
}
