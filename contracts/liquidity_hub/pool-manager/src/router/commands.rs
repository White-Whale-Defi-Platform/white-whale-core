use cosmwasm_std::{
    attr, coin, ensure, wasm_execute, Addr, BankMsg, Coin, CosmosMsg, Decimal, DepsMut,
    MessageInfo, Response, Uint128,
};
use white_whale_std::common::validate_addr_or_default;
use white_whale_std::pool_manager::{SwapOperation, SwapRoute};
use white_whale_std::whale_lair;

use crate::queries::simulate_swap_operations;
use crate::{
    state::{SwapOperations, CONFIG, SWAP_ROUTES},
    swap::perform_swap::perform_swap,
    ContractError,
};

/// Checks that the output of each [`SwapOperation`] acts as the input of the next swap.
fn assert_operations(operations: Vec<SwapOperation>) -> Result<(), ContractError> {
    // check that the output of each swap is the input of the next swap
    let mut previous_output_info = operations
        .first()
        .ok_or(ContractError::NoSwapOperationsProvided)?
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
    receiver: Option<String>,
    max_spread: Option<Decimal>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // check if the swap feature is enabled
    ensure!(
        config.feature_toggle.swaps_enabled,
        ContractError::OperationDisabled("swap".to_string())
    );

    // ensure that there was at least one operation
    // and retrieve the output token info
    let target_asset_denom = operations
        .last()
        .ok_or(ContractError::NoSwapOperationsProvided)?
        .get_target_asset_info();

    let offer_asset_denom = operations
        .first()
        .ok_or(ContractError::NoSwapOperationsProvided)?
        .get_input_asset_info();

    let offer_asset = Coin {
        denom: offer_asset_denom.to_string(),
        amount: cw_utils::must_pay(&info, offer_asset_denom)?,
    };

    assert_operations(operations.clone())?;

    // we return the output to the sender if no alternative recipient was specified.
    let receiver =
        validate_addr_or_default(&deps.as_ref(), receiver, info.sender.clone()).to_string();

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
                token_out_denom,
                pool_identifier,
                ..
            } => {
                // inside assert_operations() we have already checked that
                // the output of each swap is the input of the next swap.

                let swap_result = perform_swap(
                    deps.branch(),
                    previous_swap_output.clone(),
                    token_out_denom,
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
                if !swap_result.protocol_fee_asset.amount.is_zero() {
                    fee_messages.push(
                        wasm_execute(
                            config.bonding_manager_addr.to_string(),
                            &whale_lair::ExecuteMsg::FillRewards {
                                assets: vec![swap_result.protocol_fee_asset.clone()],
                            },
                            vec![swap_result.protocol_fee_asset.clone()],
                        )?
                        .into(),
                    );
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
            to_address: receiver.clone(),
            amount: vec![coin(receiver_balance.u128(), target_asset_denom.clone())],
        })
        .add_messages(fee_messages)
        .add_attributes(vec![
            attr("action", "execute_swap_operations".to_string()),
            attr("sender", info.sender.to_string()),
            attr("receiver", receiver),
            attr("offer_info", offer_asset.denom),
            attr("offer_amount", offer_asset.amount.to_string()),
            attr("return_denom", target_asset_denom),
            attr("return_amount", receiver_balance.to_string()),
        ])
        .add_attributes(swap_attributes))
}

pub fn add_swap_routes(
    deps: DepsMut,
    sender: Addr,
    swap_routes: Vec<SwapRoute>,
) -> Result<Response, ContractError> {
    let mut attributes = vec![];

    for swap_route in swap_routes {
        simulate_swap_operations(
            deps.as_ref(),
            Uint128::one(),
            swap_route.clone().swap_operations,
        )
        .map_err(|_| ContractError::InvalidSwapRoute(swap_route.clone()))?;

        let swap_route_key =
            SWAP_ROUTES.key((&swap_route.offer_asset_denom, &swap_route.ask_asset_denom));

        // Add the swap route if it does not exist, otherwise return an error
        if swap_route_key.may_load(deps.storage)?.is_some() {
            return Err(ContractError::SwapRouteAlreadyExists {
                offer_asset: swap_route.offer_asset_denom,
                ask_asset: swap_route.ask_asset_denom,
            });
        }
        swap_route_key.save(
            deps.storage,
            &SwapOperations {
                creator: sender.to_string(),
                swap_operations: swap_route.clone().swap_operations,
            },
        )?;

        attributes.push(attr("swap_route", swap_route.clone().to_string()));
    }

    Ok(Response::new()
        .add_attribute("action", "add_swap_routes")
        .add_attributes(attributes))
}

pub fn remove_swap_routes(
    deps: DepsMut,
    sender: Addr,
    swap_routes: Vec<SwapRoute>,
) -> Result<Response, ContractError> {
    let mut attributes = vec![];

    for swap_route in swap_routes {
        let swap_route_key =
            SWAP_ROUTES.key((&swap_route.offer_asset_denom, &swap_route.ask_asset_denom));

        // Remove the swap route if it exists
        if swap_route_key.has(deps.storage) {
            // only contract owner or route creator can remove the swap route
            let creator = swap_route_key.load(deps.storage)?.creator;
            if !cw_ownable::is_owner(deps.storage, &sender)? && sender != creator {
                return Err(ContractError::Unauthorized);
            }
            swap_route_key.remove(deps.storage);
            attributes.push(attr("swap_route", swap_route.clone().to_string()));
        } else {
            return Err(ContractError::NoSwapRouteForAssets {
                offer_asset: swap_route.offer_asset_denom,
                ask_asset: swap_route.ask_asset_denom,
            });
        }
    }

    Ok(Response::new()
        .add_attribute("action", "remove_swap_routes")
        .add_attributes(attributes))
}
