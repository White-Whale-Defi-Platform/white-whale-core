use crate::{state::CONFIG, ContractError};
use cosmwasm_std::{ensure, Addr, BankMsg, CosmosMsg, DepsMut, MessageInfo, Response};

pub const MAX_ASSETS_PER_POOL: usize = 4;

use crate::state::get_pool_by_identifier;
use cosmwasm_std::Decimal;
use white_whale_std::coin::burn_coin_msg;
use white_whale_std::common::validate_addr_or_default;

use super::perform_swap::perform_swap;

#[allow(clippy::too_many_arguments)]
pub fn swap(
    mut deps: DepsMut,
    info: MessageInfo,
    sender: Addr,
    ask_asset_denom: String,
    belief_price: Option<Decimal>,
    max_spread: Option<Decimal>,
    receiver: Option<String>,
    pool_identifier: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // check if the swap feature is enabled
    ensure!(
        config.feature_toggle.swaps_enabled,
        ContractError::OperationDisabled("swap".to_string())
    );

    let offer_asset = cw_utils::one_coin(&info)?;

    // verify that the assets sent match the ones from the pool
    let pool = get_pool_by_identifier(&deps.as_ref(), &pool_identifier)?;
    ensure!(
        [ask_asset_denom.clone(), offer_asset.denom.clone()]
            .iter()
            .all(|asset| pool
                .assets
                .iter()
                .any(|pool_asset| pool_asset.denom == *asset)),
        ContractError::AssetMismatch
    );

    // perform the swap
    let swap_result = perform_swap(
        deps.branch(),
        offer_asset.clone(),
        ask_asset_denom,
        pool_identifier,
        belief_price,
        max_spread,
    )?;

    // add messages
    let mut messages: Vec<CosmosMsg> = vec![];

    let receiver = validate_addr_or_default(&deps.as_ref(), receiver, info.sender);

    if !swap_result.return_asset.amount.is_zero() {
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: receiver.clone().into_string(),
            amount: vec![swap_result.return_asset.clone()],
        }));
    }

    if !swap_result.burn_fee_asset.amount.is_zero() {
        messages.push(burn_coin_msg(swap_result.burn_fee_asset.clone()));
    }

    if !swap_result.protocol_fee_asset.amount.is_zero() {
        messages.push(white_whale_std::bonding_manager::fill_rewards_msg(
            config.bonding_manager_addr.to_string(),
            vec![swap_result.protocol_fee_asset.clone()],
        )?);
    }

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("action", "swap".to_string()),
        ("sender", sender.into_string()),
        ("receiver", receiver.into_string()),
        ("offer_denom", offer_asset.denom),
        ("ask_denom", swap_result.return_asset.denom),
        ("offer_amount", offer_asset.amount.to_string()),
        ("return_amount", swap_result.return_asset.amount.to_string()),
        ("spread_amount", swap_result.spread_amount.to_string()),
        (
            "swap_fee_amount",
            swap_result.swap_fee_asset.amount.to_string(),
        ),
        (
            "protocol_fee_amount",
            swap_result.protocol_fee_asset.amount.to_string(),
        ),
        (
            "burn_fee_amount",
            swap_result.burn_fee_asset.amount.to_string(),
        ),
        #[cfg(feature = "osmosis")]
        (
            "osmosis_fee_amount",
            swap_result.osmosis_fee_asset.amount.to_string(),
        ),
        (
            "swap_type",
            swap_result.pool_info.pool_type.get_label().to_string(),
        ),
    ]))
}
