use cosmwasm_std::{
    attr, coin, Addr, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, MessageInfo, Response, Uint128,
};
use white_whale_std::pool_manager::SwapOperation;

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
    let target_asset_denom = operations
        .last()
        .ok_or(ContractError::NoSwapOperationsProvided {})?
        .get_target_asset_info();

    let offer_asset_denom = operations
        .first()
        .ok_or(ContractError::NoSwapOperationsProvided {})?
        .get_input_asset_info();

    let offer_asset = Coin {
        denom: offer_asset_denom.to_string(),
        amount: cw_utils::must_pay(&info, offer_asset_denom)?,
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
                // TODO: do we need to use token_in_denom?
                token_in_denom: _,
                pool_identifier,
                ..
            } => {
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
                    fee_messages.push(CosmosMsg::Bank(BankMsg::Burn {
                        amount: vec![swap_result.burn_fee_asset],
                    }));
                }

                // todo this should be not a BankMsg but a fill_rewards msg
                if !swap_result.protocol_fee_asset.amount.is_zero() {
                    fee_messages.push(CosmosMsg::Bank(BankMsg::Send {
                        to_address: config.whale_lair_addr.to_string(),
                        amount: vec![swap_result.protocol_fee_asset],
                    }));
                }

                // todo remove, the swap_fee_asset stays in the pool
                if !swap_result.swap_fee_asset.amount.is_zero() {
                    fee_messages.push(CosmosMsg::Bank(BankMsg::Send {
                        to_address: config.whale_lair_addr.to_string(),
                        amount: vec![swap_result.swap_fee_asset],
                    }));
                }
            }
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
            amount: vec![coin(receiver_balance.u128(), target_asset_denom.clone())],
        })
        .add_messages(fee_messages)
        .add_attributes(vec![
            attr("action", "execute_swap_operations"),
            attr("sender", info.sender.as_str()),
            attr("receiver", to.as_str()),
            attr("offer_info", offer_asset.denom),
            attr("offer_amount", offer_asset.amount),
            attr("return_denom", &target_asset_denom),
            attr("return_amount", receiver_balance.to_string()),
        ])
        .add_attributes(swap_attributes))
}
