use std::cmp::Ordering;

use cosmwasm_std::{
    ensure, BankMsg, Coin, CosmosMsg, Decimal, MessageInfo, OverflowError, OverflowOperation,
    Uint128,
};

use white_whale_std::incentive_manager::{Config, IncentiveParams, DEFAULT_INCENTIVE_DURATION};

use crate::ContractError;

/// Processes the incentive creation fee and returns the appropriate messages to be sent
pub(crate) fn process_incentive_creation_fee(
    config: &Config,
    info: &MessageInfo,
    incentive_creation_fee: &Coin,
    params: &IncentiveParams,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let mut messages: Vec<CosmosMsg> = vec![];

    // verify the fee to create an incentive is being paid
    let paid_fee_amount = info
        .funds
        .iter()
        .find(|coin| coin.denom == incentive_creation_fee.denom)
        .ok_or(ContractError::IncentiveFeeMissing)?
        .amount;

    match paid_fee_amount.cmp(&incentive_creation_fee.amount) {
        Ordering::Equal => (), // do nothing if user paid correct amount,
        Ordering::Less => {
            // user underpaid
            return Err(ContractError::IncentiveFeeNotPaid {
                paid_amount: paid_fee_amount,
                required_amount: incentive_creation_fee.amount,
            });
        }
        Ordering::Greater => {
            // if the user is paying more than the incentive_creation_fee, check if it's trying to create
            // an incentive with the same asset as the incentive_creation_fee.
            // otherwise, refund the difference
            if incentive_creation_fee.denom == params.incentive_asset.denom {
                // check if the amounts add up, i.e. the fee + incentive asset = paid amount. That is because the incentive asset
                // and the creation fee asset are the same, all go in the info.funds of the transaction

                ensure!(
                    params
                        .incentive_asset
                        .amount
                        .checked_add(incentive_creation_fee.amount)?
                        == paid_fee_amount,
                    ContractError::AssetMismatch
                );
            } else {
                let refund_amount = paid_fee_amount.saturating_sub(incentive_creation_fee.amount);

                if refund_amount > Uint128::zero() {
                    messages.push(
                        BankMsg::Send {
                            to_address: info.sender.clone().into_string(),
                            amount: vec![Coin {
                                amount: refund_amount,
                                denom: incentive_creation_fee.denom.clone(),
                            }],
                        }
                        .into(),
                    );
                }
            }
        }
    }

    // send incentive creation fee to whale lair for distribution
    messages.push(white_whale_std::whale_lair::fill_rewards_msg_coin(
        config.whale_lair_addr.clone().into_string(),
        vec![incentive_creation_fee.to_owned()],
    )?);

    Ok(messages)
}

/// Asserts the incentive asset was sent correctly, considering the incentive creation fee if applicable.
pub(crate) fn assert_incentive_asset(
    info: &MessageInfo,
    incentive_creation_fee: &Coin,
    params: &IncentiveParams,
) -> Result<(), ContractError> {
    let coin_sent = info
        .funds
        .iter()
        .find(|sent| sent.denom == params.incentive_asset.denom)
        .ok_or(ContractError::AssetMismatch)?;

    if incentive_creation_fee.denom != params.incentive_asset.denom {
        ensure!(
            coin_sent.amount == params.incentive_asset.amount,
            ContractError::AssetMismatch
        );
    } else {
        ensure!(
            params
                .incentive_asset
                .amount
                .checked_add(incentive_creation_fee.amount)?
                == coin_sent.amount,
            ContractError::AssetMismatch
        );
    }

    Ok(())
}

/// Validates the incentive epochs. Returns a tuple of (start_epoch, end_epoch) for the incentive.
pub(crate) fn validate_incentive_epochs(
    params: &IncentiveParams,
    current_epoch: u64,
    max_incentive_epoch_buffer: u64,
) -> Result<(u64, u64), ContractError> {
    // assert epoch params are correctly set
    let start_epoch = params.start_epoch.unwrap_or(current_epoch);

    let preliminary_end_epoch = params.preliminary_end_epoch.unwrap_or(
        start_epoch
            .checked_add(DEFAULT_INCENTIVE_DURATION)
            .ok_or(ContractError::InvalidEndEpoch)?,
    );

    // ensure that start date is before end date
    ensure!(
        start_epoch < preliminary_end_epoch,
        ContractError::IncentiveStartTimeAfterEndTime
    );

    // ensure the incentive is set to end in a future epoch
    ensure!(
        preliminary_end_epoch > current_epoch,
        ContractError::IncentiveEndsInPast
    );

    // ensure that start date is set within buffer
    ensure!(
        start_epoch
            <= current_epoch
                .checked_add(max_incentive_epoch_buffer)
                .ok_or(ContractError::OverflowError(OverflowError {
                    operation: OverflowOperation::Add,
                    operand1: current_epoch.to_string(),
                    operand2: max_incentive_epoch_buffer.to_string(),
                }))?,
        ContractError::IncentiveStartTooFar
    );

    Ok((start_epoch, preliminary_end_epoch))
}

/// Validates the emergency unlock penalty is within the allowed range (0-100%). Returns value it's validating, i.e. the penalty.
pub(crate) fn validate_emergency_unlock_penalty(
    emergency_unlock_penalty: Decimal,
) -> Result<Decimal, ContractError> {
    ensure!(
        emergency_unlock_penalty <= Decimal::percent(100),
        ContractError::InvalidEmergencyUnlockPenalty
    );

    Ok(emergency_unlock_penalty)
}
