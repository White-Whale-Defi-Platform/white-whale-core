use std::prelude::rust_2015::Ok;

use cosmwasm_std::{to_binary, CosmosMsg, StdResult, Uint128, WasmMsg};

#[cfg(any(feature = "token_factory", feature = "osmosis_token_factory"))]
use crate::pool_network::asset::is_factory_token;
#[cfg(feature = "token_factory")]
use crate::pool_network::denom::{Coin, MsgBurn, MsgMint};
#[cfg(feature = "osmosis_token_factory")]
use crate::pool_network::denom_osmosis::{Coin, MsgBurn, MsgMint};
#[cfg(any(feature = "token_factory", feature = "osmosis_token_factory"))]
use cosmwasm_std::coins;

/// Creates the Mint LP message
#[allow(unused_variables)]
pub fn mint_lp_token_msg(
    liquidity_asset: String,
    recipient: String,
    sender: String,
    amount: Uint128,
) -> StdResult<Vec<CosmosMsg>> {
    #[cfg(any(feature = "token_factory", feature = "osmosis_token_factory"))]
    if is_factory_token(liquidity_asset.as_str()) {
        let mut messages = vec![];
        messages.push(<MsgMint as Into<CosmosMsg>>::into(MsgMint {
            sender: sender.clone(),
            amount: Some(Coin {
                denom: liquidity_asset.clone(),
                amount: amount.to_string(),
            }),
        }));

        if sender != recipient {
            messages.push(CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
                to_address: recipient,
                amount: coins(amount.u128(), liquidity_asset.as_str()),
            }));
        }

        Ok(messages)
    } else {
        Ok(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: liquidity_asset,
            msg: to_binary(&cw20::Cw20ExecuteMsg::Mint { recipient, amount })?,
            funds: vec![],
        })])
    }

    #[cfg(all(not(feature = "token_factory"), not(feature = "osmosis_token_factory")))]
    Ok(vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: liquidity_asset,
        msg: to_binary(&cw20::Cw20ExecuteMsg::Mint { recipient, amount })?,
        funds: vec![],
    })])
}

/// Creates the Burn LP message
#[allow(unused_variables)]
pub fn burn_lp_asset_msg(
    liquidity_asset: String,
    sender: String,
    amount: Uint128,
) -> StdResult<CosmosMsg> {
    #[cfg(any(feature = "token_factory", feature = "osmosis_token_factory"))]
    if is_factory_token(liquidity_asset.as_str()) {
        Ok(<MsgBurn as Into<CosmosMsg>>::into(MsgBurn {
            sender,
            amount: Some(Coin {
                denom: liquidity_asset,
                amount: amount.to_string(),
            }),
        }))
    } else {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: liquidity_asset,
            msg: to_binary(&cw20::Cw20ExecuteMsg::Burn { amount })?,
            funds: vec![],
        }))
    }
    #[cfg(all(not(feature = "token_factory"), not(feature = "osmosis_token_factory")))]
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: liquidity_asset,
        msg: to_binary(&cw20::Cw20ExecuteMsg::Burn { amount })?,
        funds: vec![],
    }))
}
