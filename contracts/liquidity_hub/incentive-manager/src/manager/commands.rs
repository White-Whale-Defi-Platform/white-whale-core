use cosmwasm_std::{
    ensure, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, StdError,
    Storage, Uint128,
};

use white_whale_std::coin::{get_subdenom, is_factory_token};
use white_whale_std::epoch_manager::common::validate_epoch;
use white_whale_std::epoch_manager::hooks::EpochChangedHookMsg;
use white_whale_std::incentive_manager::MIN_INCENTIVE_AMOUNT;
use white_whale_std::incentive_manager::{Curve, Incentive, IncentiveParams};

use crate::helpers::{
    assert_incentive_asset, process_incentive_creation_fee, validate_emergency_unlock_penalty,
    validate_incentive_epochs,
};
use crate::state::{
    get_incentive_by_identifier, get_incentives_by_lp_asset, CONFIG, INCENTIVES, INCENTIVE_COUNTER,
    LP_WEIGHTS_HISTORY,
};
use crate::ContractError;

pub(crate) fn fill_incentive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    params: IncentiveParams,
) -> Result<Response, ContractError> {
    // if an incentive_identifier was passed in the params, check if an incentive with such identifier
    // exists and if the sender is allow to refill it, otherwise create a new incentive
    if let Some(incentive_indentifier) = params.clone().incentive_identifier {
        let incentive_result = get_incentive_by_identifier(deps.storage, &incentive_indentifier);

        if let Ok(incentive) = incentive_result {
            // the incentive exists, try to expand it
            return expand_incentive(deps, env, info, incentive, params);
        }
        // the incentive does not exist, try to create it
    }

    // if no identifier was passed in the params or if the incentive does not exist, try to create the incentive
    create_incentive(deps, env, info, params)
}

/// Creates an incentive with the given params
fn create_incentive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mut params: IncentiveParams,
) -> Result<Response, ContractError> {
    // check if there are any expired incentives for this LP asset
    let config = CONFIG.load(deps.storage)?;
    let incentives = get_incentives_by_lp_asset(
        deps.storage,
        &params.lp_denom,
        None,
        Some(config.max_concurrent_incentives),
    )?;

    let current_epoch = white_whale_std::epoch_manager::common::get_current_epoch(
        deps.as_ref(),
        config.epoch_manager_addr.clone().into_string(),
    )?;
    validate_epoch(&current_epoch, env.block.time)?;

    let (expired_incentives, incentives): (Vec<_>, Vec<_>) = incentives
        .into_iter()
        .partition(|incentive| incentive.is_expired(current_epoch.id));

    let mut messages: Vec<CosmosMsg> = vec![];

    // close expired incentives if there are any
    if !expired_incentives.is_empty() {
        messages.append(&mut close_incentives(deps.storage, expired_incentives)?);
    }

    // check if more incentives can be created for this particular LP asset
    ensure!(
        incentives.len() < config.max_concurrent_incentives as usize,
        ContractError::TooManyIncentives {
            max: config.max_concurrent_incentives,
        }
    );

    // check the incentive is being created with a valid amount
    ensure!(
        params.incentive_asset.amount >= MIN_INCENTIVE_AMOUNT,
        ContractError::InvalidIncentiveAmount {
            min: MIN_INCENTIVE_AMOUNT.u128()
        }
    );

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
    assert_incentive_asset(&info, &incentive_creation_fee, &params)?;

    // assert epoch params are correctly set
    let (start_epoch, preliminary_end_epoch) = validate_incentive_epochs(
        &params,
        current_epoch.id,
        u64::from(config.max_incentive_epoch_buffer),
    )?;

    // create incentive identifier
    let incentive_id = INCENTIVE_COUNTER
        .update::<_, StdError>(deps.storage, |current_id| Ok(current_id + 1u64))?;
    let incentive_identifier = params
        .incentive_identifier
        .unwrap_or(incentive_id.to_string());

    // make sure another incentive with the same identifier doesn't exist
    ensure!(
        get_incentive_by_identifier(deps.storage, &incentive_identifier).is_err(),
        ContractError::IncentiveAlreadyExists
    );
    // the incentive does not exist, all good, continue

    // calculates the emission rate
    let emission_rate = params
        .incentive_asset
        .amount
        .checked_div_floor((preliminary_end_epoch.saturating_sub(start_epoch), 1u64))?;

    // create the incentive
    let incentive = Incentive {
        identifier: incentive_identifier,
        start_epoch,
        preliminary_end_epoch,
        curve: params.curve.unwrap_or(Curve::Linear),
        incentive_asset: params.incentive_asset,
        lp_denom: params.lp_denom,
        owner: info.sender,
        claimed_amount: Uint128::zero(),
        emission_rate,
        last_epoch_claimed: current_epoch.id - 1,
    };

    INCENTIVES.save(deps.storage, &incentive.identifier, &incentive)?;

    Ok(Response::default().add_attributes(vec![
        ("action", "create_incentive".to_string()),
        ("incentive_creator", incentive.owner.to_string()),
        ("incentive_identifier", incentive.identifier),
        ("start_epoch", incentive.start_epoch.to_string()),
        (
            "preliminary_end_epoch",
            incentive.preliminary_end_epoch.to_string(),
        ),
        ("emission_rate", emission_rate.to_string()),
        ("curve", incentive.curve.to_string()),
        ("incentive_asset", incentive.incentive_asset.to_string()),
        ("lp_denom", incentive.lp_denom),
    ]))
}

/// Closes an incentive. If the incentive has expired, anyone can close it. Otherwise, only the
/// incentive creator or the owner of the contract can close an incentive.
pub(crate) fn close_incentive(
    deps: DepsMut,
    info: MessageInfo,
    incentive_identifier: String,
) -> Result<Response, ContractError> {
    cw_utils::nonpayable(&info)?;

    // validate that user is allowed to close the incentive. Only the incentive creator or the owner of the contract can close an incentive
    let config = CONFIG.load(deps.storage)?;
    let current_epoch = white_whale_std::epoch_manager::common::get_current_epoch(
        deps.as_ref(),
        config.epoch_manager_addr.into_string(),
    )?;

    let incentive = get_incentive_by_identifier(deps.storage, &incentive_identifier)?;

    if !(!incentive.is_expired(current_epoch.id)
        && (incentive.owner == info.sender || cw_ownable::is_owner(deps.storage, &info.sender)?))
    {
        return Err(ContractError::Unauthorized);
    }

    Ok(Response::default()
        .add_messages(close_incentives(deps.storage, vec![incentive])?)
        .add_attributes(vec![
            ("action", "close_incentive".to_string()),
            ("incentive_identifier", incentive_identifier),
        ]))
}

/// Closes a list of incentives. Does not validate the sender, do so before calling this function.
fn close_incentives(
    storage: &mut dyn Storage,
    incentives: Vec<Incentive>,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let mut messages: Vec<CosmosMsg> = vec![];

    for mut incentive in incentives {
        // remove the incentive from the storage
        INCENTIVES.remove(storage, &incentive.identifier)?;

        // return the available asset, i.e. the amount that hasn't been claimed
        incentive.incentive_asset.amount = incentive
            .incentive_asset
            .amount
            .saturating_sub(incentive.claimed_amount);

        messages.push(
            BankMsg::Send {
                to_address: incentive.owner.into_string(),
                amount: vec![incentive.incentive_asset],
            }
            .into(),
        );
    }

    Ok(messages)
}

/// Expands an incentive with the given params
fn expand_incentive(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    mut incentive: Incentive,
    params: IncentiveParams,
) -> Result<Response, ContractError> {
    // only the incentive owner can expand it
    ensure!(incentive.owner == info.sender, ContractError::Unauthorized);

    let config = CONFIG.load(deps.storage)?;
    let current_epoch = white_whale_std::epoch_manager::common::get_current_epoch(
        deps.as_ref(),
        config.epoch_manager_addr.into_string(),
    )?;

    // check if the incentive has already expired, can't be expanded
    ensure!(
        incentive.is_expired(current_epoch.id),
        ContractError::IncentiveAlreadyExpired
    );

    // check that the asset sent matches the asset expected
    ensure!(
        incentive.incentive_asset.denom == params.incentive_asset.denom,
        ContractError::AssetMismatch
    );

    // increase the total amount of the incentive
    incentive.incentive_asset.amount = incentive
        .incentive_asset
        .amount
        .checked_add(params.incentive_asset.amount)?;
    INCENTIVES.save(deps.storage, &incentive.identifier, &incentive)?;

    Ok(Response::default().add_attributes(vec![
        ("action", "expand_incentive".to_string()),
        ("incentive_identifier", incentive.identifier),
        ("expanded_by", params.incentive_asset.to_string()),
        ("total_incentive", incentive.incentive_asset.to_string()),
    ]))
}

/// EpochChanged hook implementation. Updates the LP_WEIGHTS.
pub(crate) fn on_epoch_changed(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: EpochChangedHookMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // only the epoch manager can trigger this
    ensure!(
        info.sender == config.epoch_manager_addr,
        ContractError::Unauthorized
    );

    // get all LP tokens and update the LP_WEIGHTS_HISTORY
    let lp_assets = deps
        .querier
        .query_all_balances(env.contract.address)?
        .into_iter()
        .filter(|asset| {
            if is_factory_token(asset.denom.as_str()) {
                //todo remove this hardcoded uLP and point to the pool manager const
                get_subdenom(asset.denom.as_str()) == "uLP"
            } else {
                false
            }
        })
        .collect::<Vec<Coin>>();

    for lp_asset in &lp_assets {
        LP_WEIGHTS_HISTORY.save(
            deps.storage,
            (&lp_asset.denom, msg.current_epoch.id),
            &lp_asset.amount,
        )?;
    }

    Ok(Response::default().add_attributes(vec![
        ("action", "on_epoch_changed".to_string()),
        ("epoch", msg.current_epoch.to_string()),
    ]))
}

#[allow(clippy::too_many_arguments)]
/// Updates the configuration of the contract
pub(crate) fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    whale_lair_addr: Option<String>,
    epoch_manager_addr: Option<String>,
    create_incentive_fee: Option<Coin>,
    max_concurrent_incentives: Option<u32>,
    max_incentive_epoch_buffer: Option<u32>,
    min_unlocking_duration: Option<u64>,
    max_unlocking_duration: Option<u64>,
    emergency_unlock_penalty: Option<Decimal>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut config = CONFIG.load(deps.storage)?;

    if let Some(whale_lair_addr) = whale_lair_addr {
        config.whale_lair_addr = deps.api.addr_validate(&whale_lair_addr)?;
    }

    if let Some(epoch_manager_addr) = epoch_manager_addr {
        config.epoch_manager_addr = deps.api.addr_validate(&epoch_manager_addr)?;
    }

    if let Some(create_incentive_fee) = create_incentive_fee {
        config.create_incentive_fee = create_incentive_fee;
    }

    if let Some(max_concurrent_incentives) = max_concurrent_incentives {
        if max_concurrent_incentives == 0u32 {
            return Err(ContractError::UnspecifiedConcurrentIncentives);
        }

        config.max_concurrent_incentives = max_concurrent_incentives;
    }

    if let Some(max_incentive_epoch_buffer) = max_incentive_epoch_buffer {
        config.max_incentive_epoch_buffer = max_incentive_epoch_buffer;
    }

    if let Some(max_unlocking_duration) = max_unlocking_duration {
        if max_unlocking_duration < config.min_unlocking_duration {
            return Err(ContractError::InvalidUnbondingRange {
                min: config.min_unlocking_duration,
                max: max_unlocking_duration,
            });
        }

        config.max_unlocking_duration = max_unlocking_duration;
    }

    if let Some(min_unlocking_duration) = min_unlocking_duration {
        if config.max_unlocking_duration < min_unlocking_duration {
            return Err(ContractError::InvalidUnbondingRange {
                min: min_unlocking_duration,
                max: config.max_unlocking_duration,
            });
        }

        config.min_unlocking_duration = min_unlocking_duration;
    }

    if let Some(emergency_unlock_penalty) = emergency_unlock_penalty {
        config.emergency_unlock_penalty =
            validate_emergency_unlock_penalty(emergency_unlock_penalty)?;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default().add_attributes(vec![
        ("action", "update_config".to_string()),
        ("whale_lair_addr", config.whale_lair_addr.to_string()),
        ("epoch_manager_addr", config.epoch_manager_addr.to_string()),
        ("create_flow_fee", config.create_incentive_fee.to_string()),
        (
            "max_concurrent_flows",
            config.max_concurrent_incentives.to_string(),
        ),
        (
            "max_flow_epoch_buffer",
            config.max_incentive_epoch_buffer.to_string(),
        ),
        (
            "min_unbonding_duration",
            config.min_unlocking_duration.to_string(),
        ),
        (
            "max_unbonding_duration",
            config.max_unlocking_duration.to_string(),
        ),
        (
            "emergency_unlock_penalty",
            config.emergency_unlock_penalty.to_string(),
        ),
    ]))
}
