#[cfg(any(
    feature = "token_factory",
    feature = "osmosis_token_factory",
    feature = "injective"
))]
use cosmwasm_std::{coins, Coin};
use cosmwasm_std::{Addr, CosmosMsg, StdResult, Uint128, WasmMsg, to_json_binary};

#[cfg(any(
    feature = "token_factory",
    feature = "osmosis_token_factory",
    feature = "injective"
))]
use crate::pool_network::asset::is_factory_token;
#[cfg(any(
    feature = "token_factory",
    feature = "osmosis_token_factory",
    feature = "injective"
))]
use crate::tokenfactory;

/// Creates the Mint LP message
#[allow(unused_variables)]
pub fn mint_lp_token_msg(
    liquidity_asset: String,
    recipient: &Addr,
    sender: &Addr,
    amount: Uint128,
) -> StdResult<Vec<CosmosMsg>> {
    #[cfg(any(
        feature = "token_factory",
        feature = "osmosis_token_factory",
        feature = "injective"
    ))]
    if is_factory_token(liquidity_asset.as_str()) {
        let mut messages = vec![];
        messages.push(tokenfactory::mint::mint(
            sender.clone(),
            Coin {
                denom: liquidity_asset.clone(),
                amount,
            },
        ));

        if sender != recipient {
            messages.push(CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
                to_address: recipient.clone().into_string(),
                amount: coins(amount.u128(), liquidity_asset.as_str()),
            }));
        }

        return Ok(messages);
    }

    Ok(vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: liquidity_asset,
        msg: to_json_binary(&cw20::Cw20ExecuteMsg::Mint {
            recipient: recipient.clone().into_string(),
            amount,
        })?,
        funds: vec![],
    })])
}

/// Creates the Burn LP message
#[allow(unused_variables)]
pub fn burn_lp_asset_msg(
    liquidity_asset: String,
    sender: Addr,
    amount: Uint128,
) -> StdResult<CosmosMsg> {
    #[cfg(any(
        feature = "token_factory",
        feature = "osmosis_token_factory",
        feature = "injective"
    ))]
    if is_factory_token(liquidity_asset.as_str()) {
        return Ok(tokenfactory::burn::burn(
            sender.clone(),
            Coin {
                denom: liquidity_asset.clone(),
                amount,
            },
        ));
    }

    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: liquidity_asset,
        msg: to_json_binary(&cw20::Cw20ExecuteMsg::Burn { amount })?,
        funds: vec![],
    }))
}
