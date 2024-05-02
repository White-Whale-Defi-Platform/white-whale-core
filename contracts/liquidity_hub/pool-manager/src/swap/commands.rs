use crate::{state::CONFIG, ContractError};
use cosmwasm_std::{
    ensure, wasm_execute, Addr, BankMsg, CosmosMsg, DepsMut, Env, MessageInfo, Response,
};

pub const MAX_ASSETS_PER_POOL: usize = 4;

use crate::state::get_pair_by_identifier;
use cosmwasm_std::Decimal;
use white_whale_std::common::validate_addr_or_default;
use white_whale_std::whale_lair;

use super::perform_swap::perform_swap;

#[allow(clippy::too_many_arguments)]
pub fn swap(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    sender: Addr,
    ask_asset_denom: String,
    belief_price: Option<Decimal>,
    max_spread: Option<Decimal>,
    receiver: Option<String>,
    pair_identifier: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // check if the swap feature is enabled
    ensure!(
        config.feature_toggle.swaps_enabled,
        ContractError::OperationDisabled("swap".to_string())
    );

    let offer_asset = cw_utils::one_coin(&info)?;

    // verify that the assets sent match the ones from the pool
    let pair = get_pair_by_identifier(&deps.as_ref(), &pair_identifier)?;
    ensure!(
        vec![ask_asset_denom, offer_asset.denom.clone()]
            .iter()
            .all(|asset| pair
                .assets
                .iter()
                .any(|pool_asset| pool_asset.denom == *asset)),
        ContractError::AssetMismatch {}
    );

    // perform the swap
    let swap_result = perform_swap(
        deps.branch(),
        offer_asset.clone(),
        pair_identifier,
        belief_price,
        max_spread,
    )?;

    // add messages
    let mut messages: Vec<CosmosMsg> = vec![];

    let receiver = validate_addr_or_default(&deps.as_ref(), receiver, info.sender);

    // first we add the swap result
    if !swap_result.return_asset.amount.is_zero() {
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: receiver.clone().into_string(),
            amount: vec![swap_result.return_asset.clone()],
        }));
    }

    // then we add the fees
    if !swap_result.burn_fee_asset.amount.is_zero() {
        messages.push(CosmosMsg::Bank(BankMsg::Burn {
            amount: vec![swap_result.burn_fee_asset.clone()],
        }));
    }
    if !swap_result.protocol_fee_asset.amount.is_zero() {
        messages.push(
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
            swap_result.osmosis_fee_amount.to_string(),
        ),
        (
            "swap_type",
            swap_result.pair_info.pair_type.get_label().to_string(),
        ),
    ]))
}
