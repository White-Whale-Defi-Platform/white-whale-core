use cosmwasm_std::{CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError, Uint128};
use std::collections::HashMap;

use white_whale::epoch_manager::epoch_manager::EpochResponse;
use white_whale::incentive_manager::{Curve, Incentive, IncentiveParams};

use crate::helpers::{assert_incentive_asset, process_incentive_creation_fee};
use crate::state::{get_incentives_by_lp_asset, CONFIG, INCENTIVE_COUNTER};
use crate::ContractError;

/// Minimum amount of an asset to create an incentive with
pub const MIN_INCENTIVE_AMOUNT: Uint128 = Uint128::new(1_000u128);

/// Default incentive duration in epochs
pub const DEFAULT_INCENTIVE_DURATION: u64 = 14u64;

/// Creates an incentive with the given params
pub(crate) fn create_incentive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mut params: IncentiveParams,
) -> Result<Response, ContractError> {
    // check if more incentives can be created for this particular LP asset
    let config = CONFIG.load(deps.storage)?;
    let incentives = get_incentives_by_lp_asset(
        deps.storage,
        &params.lp_asset,
        None,
        Some(config.max_concurrent_incentives),
    )?;
    if incentives.len() == config.max_concurrent_incentives as usize {
        return Err(ContractError::TooManyIncentives {
            max: config.max_concurrent_incentives,
        });
    }

    // check the flow is being created with a valid amount
    if params.incentive_asset.amount < MIN_INCENTIVE_AMOUNT {
        return Err(ContractError::InvalidIncentiveAmount {
            min: MIN_INCENTIVE_AMOUNT.u128(),
        });
    }

    let mut messages: Vec<CosmosMsg> = vec![];

    let incentive_creation_fee = config.create_incentive_fee.clone();

    if incentive_creation_fee.amount != Uint128::zero() {
        // verify the fee to create an incentive is being paid
        messages.append(&mut process_incentive_creation_fee(
            &config,
            &info,
            &incentive_creation_fee,
            &mut params,
        )?);
    }

    // verify the incentive asset was sent
    messages.append(&mut assert_incentive_asset(
        deps.as_ref(),
        &env,
        &info,
        &incentive_creation_fee,
        &mut params,
    )?);

    // assert epoch params are correctly set

    let epoch_response: EpochResponse = deps.querier.query_wasm_smart(
        config.epoch_manager_addr.into_string(),
        &white_whale::epoch_manager::epoch_manager::QueryMsg::CurrentEpoch {},
    )?;

    let current_epoch = epoch_response.epoch.id;

    let end_epoch = params.end_epoch.unwrap_or(
        current_epoch
            .checked_add(DEFAULT_INCENTIVE_DURATION)
            .ok_or(ContractError::InvalidEndEpoch {})?,
    );

    // ensure the incentive is set to end in a future epoch
    if current_epoch > end_epoch {
        return Err(ContractError::IncentiveEndsInPast);
    }

    let start_epoch = params.start_epoch.unwrap_or(current_epoch);

    // ensure that start date is before end date
    if start_epoch > end_epoch {
        return Err(ContractError::IncentiveStartTimeAfterEndTime);
    }

    // ensure that start date is set within buffer
    if start_epoch > current_epoch + u64::from(config.max_incentive_epoch_buffer) {
        return Err(ContractError::IncentiveStartTooFar);
    }

    // create incentive identifier
    let incentive_id = INCENTIVE_COUNTER
        .update::<_, StdError>(deps.storage, |current_id| Ok(current_id + 1u64))?;
    let incentive_identifier = params
        .incentive_indentifier
        .unwrap_or(incentive_id.to_string());

    // create the incentive
    let incentive = Incentive {
        incentive_identifier,
        start_epoch,
        end_epoch,
        emitted_tokens: HashMap::new(),
        curve: params.curve.unwrap_or(Curve::Linear),
        incentive_asset: params.incentive_asset,
        lp_asset: params.lp_asset,
        incentive_creator: info.sender,
        claimed_amount: Uint128::zero(),
        asset_history: Default::default(),
    };

    Ok(Response::default().add_attributes(vec![
        ("action", "create_incentive".to_string()),
        ("incentive_creator", incentive.incentive_creator.to_string()),
        ("incentive_identifier", incentive.incentive_identifier),
        ("start_epoch", incentive.start_epoch.to_string()),
        ("end_epoch", incentive.end_epoch.to_string()),
        ("curve", incentive.curve.to_string()),
        ("incentive_asset", incentive.incentive_asset.to_string()),
        ("lp_asset", incentive.lp_asset.to_string()),
    ]))
}
