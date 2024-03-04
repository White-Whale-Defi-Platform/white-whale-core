use cosmwasm_std::{Addr, CosmosMsg, DepsMut, Env, MessageInfo, Response};
use white_whale_std::pool_network::asset::{Asset, AssetInfo};

use crate::{state::MANAGER_CONFIG, ContractError};
#[cfg(any(feature = "token_factory", feature = "osmosis_token_factory"))]
use cosmwasm_std::coins;
#[cfg(any(feature = "token_factory", feature = "osmosis_token_factory"))]
use white_whale_std::pool_network::asset::is_factory_token;
#[cfg(feature = "token_factory")]
use white_whale_std::pool_network::denom::MsgCreateDenom;
#[cfg(feature = "osmosis_token_factory")]
use white_whale_std::pool_network::denom_osmosis::MsgCreateDenom;

#[cfg(feature = "token_factory")]
use white_whale_std::pool_network::denom::{Coin, MsgBurn, MsgMint};
#[cfg(feature = "osmosis_token_factory")]
use white_whale_std::pool_network::denom_osmosis::{Coin, MsgBurn, MsgMint};
pub const MAX_ASSETS_PER_POOL: usize = 4;
pub const LP_SYMBOL: &str = "uLP";

use cosmwasm_std::Decimal;

use super::perform_swap::perform_swap;

#[allow(clippy::too_many_arguments)]
pub fn swap(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    sender: Addr,
    offer_asset: Asset,
    _ask_asset: AssetInfo,
    belief_price: Option<Decimal>,
    max_spread: Option<Decimal>,
    to: Option<Addr>,
    pair_identifier: String,
) -> Result<Response, ContractError> {
    // ensure only native tokens are being swapped
    // in the future, we are removing cw20 tokens
    if !offer_asset.is_native_token() {
        return Err(ContractError::Unauthorized {});
    }

    let config = MANAGER_CONFIG.load(deps.storage)?;
    // check if the swap feature is enabled
    if !config.feature_toggle.swaps_enabled {
        return Err(ContractError::OperationDisabled("swap".to_string()));
    }

    offer_asset.assert_sent_native_token_balance(&info)?;

    // perform the swap
    let swap_result = perform_swap(
        deps,
        offer_asset.clone(),
        pair_identifier,
        belief_price,
        max_spread,
    )?;

    // add messages
    let mut messages: Vec<CosmosMsg> = vec![];
    let receiver = to.unwrap_or_else(|| sender.clone());

    // first we add the swap result
    if !swap_result.return_asset.amount.is_zero() {
        messages.push(
            swap_result
                .return_asset
                .clone()
                .into_msg(receiver.clone())?,
        );
    }
    // then we add the fees
    if !swap_result.burn_fee_asset.amount.is_zero() {
        messages.push(swap_result.burn_fee_asset.clone().into_burn_msg()?);
    }
    if !swap_result.protocol_fee_asset.amount.is_zero() {
        messages.push(
            swap_result
                .protocol_fee_asset
                .clone()
                .into_msg(config.fee_collector_addr.clone())?,
        );
    }
    if !swap_result.swap_fee_asset.amount.is_zero() {
        messages.push(
            swap_result
                .swap_fee_asset
                .clone()
                .into_msg(config.fee_collector_addr)?,
        );
    }

    // 1. send collateral token from the contract to a user
    // 2. stores the protocol fees
    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("action", "swap"),
        ("sender", sender.as_str()),
        ("receiver", receiver.as_str()),
        ("offer_asset", &offer_asset.info.to_string()),
        ("ask_asset", &swap_result.return_asset.info.to_string()),
        ("offer_amount", &offer_asset.amount.to_string()),
        (
            "return_amount",
            &swap_result.return_asset.amount.to_string(),
        ),
        ("spread_amount", &swap_result.spread_amount.to_string()),
        (
            "swap_fee_amount",
            &swap_result.swap_fee_asset.amount.to_string(),
        ),
        (
            "protocol_fee_amount",
            &swap_result.protocol_fee_asset.amount.to_string(),
        ),
        (
            "burn_fee_amount",
            &swap_result.burn_fee_asset.amount.to_string(),
        ),
        ("swap_type", swap_result.pair_info.pair_type.get_label()),
    ]))
}
