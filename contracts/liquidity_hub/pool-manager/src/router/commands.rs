use std::{collections::HashMap, str::FromStr};

use cosmwasm_std::{
    coin, Addr, BankMsg, Decimal, DepsMut, Env, MessageInfo, Response, StdError, Uint128,
};
use white_whale_std::{
    pool_manager::{ExecuteMsg, SwapOperation},
    pool_network::asset::{Asset, AssetInfo},
};

use crate::{swap::perform_swap::perform_swap, ContractError};

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

    assert_operations(&operations)?;

    // we return the output to the sender if no alternative recipient was specified.
    let to = to.unwrap_or(info.sender);

    // perform each swap operation
    // we start off with the initial funds
    let mut previous_swap_output = Asset {
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
    Ok(Response::new().add_message(BankMsg::Send {
        to_address: to.to_string(),
        amount: vec![coin(receiver_balance.u128(), target_denom)],
    }))
}
