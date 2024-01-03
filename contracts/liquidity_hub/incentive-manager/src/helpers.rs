use std::cmp::Ordering;

use cosmwasm_std::{wasm_execute, BankMsg, Coin, CosmosMsg, Deps, Env, MessageInfo};

use white_whale::incentive_manager::{
    Config, IncentiveParams, PositionParams, DEFAULT_INCENTIVE_DURATION,
};
use white_whale::pool_network::asset::{Asset, AssetInfo};

use crate::ContractError;

/// Processes the incentive creation fee and returns the appropriate messages to be sent
pub(crate) fn process_incentive_creation_fee(
    config: &Config,
    info: &MessageInfo,
    incentive_creation_fee: &Asset,
    params: &mut IncentiveParams,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let mut messages: Vec<CosmosMsg> = vec![];

    // verify the fee to create an incentive is being paid
    match incentive_creation_fee.info.clone() {
        AssetInfo::Token { .. } => {
            // only fees in native tokens are supported
            return Err(ContractError::FeeAssetNotSupported);
        }
        AssetInfo::NativeToken {
            denom: incentive_creation_fee_denom,
        } => {
            // check paid fee amount
            let paid_fee_amount = info
                .funds
                .iter()
                .find(|coin| coin.denom == incentive_creation_fee_denom)
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
                    // and incentive with the same asset as the incentive_creation_fee.
                    // otherwise, refund the difference
                    match params.incentive_asset.info.clone() {
                        AssetInfo::Token { .. } => {}
                        AssetInfo::NativeToken {
                            denom: incentive_asset_denom,
                        } => {
                            if incentive_creation_fee_denom == incentive_asset_denom {
                                // check if the amounts add up, i.e. the fee + incentive asset = paid amount. That is because the incentive asset
                                // and the creation fee asset are the same, all go in the info.funds of the transaction
                                if params
                                    .incentive_asset
                                    .amount
                                    .checked_add(incentive_creation_fee.amount)?
                                    != paid_fee_amount
                                {
                                    return Err(ContractError::AssetMismatch);
                                }
                            } else {
                                messages.push(
                                    BankMsg::Send {
                                        to_address: info.sender.clone().into_string(),
                                        amount: vec![Coin {
                                            amount: paid_fee_amount - incentive_creation_fee.amount,
                                            denom: incentive_creation_fee_denom.clone(),
                                        }],
                                    }
                                    .into(),
                                );
                            }
                        }
                    }
                }
            }

            // send incentive creation fee to whale lair for distribution
            messages.push(white_whale::whale_lair::fill_rewards_msg(
                config.whale_lair_addr.clone().into_string(),
                vec![incentive_creation_fee.to_owned()],
            )?);
        }
    }

    Ok(messages)
}

/// Asserts the incentive asset was sent correctly, considering the incentive creation fee if applicable.
/// Returns a vector of messages to be sent (applies only when the incentive asset is a CW20 token)
pub(crate) fn assert_incentive_asset(
    deps: Deps,
    env: &Env,
    info: &MessageInfo,
    incentive_creation_fee: &Asset,
    params: &mut IncentiveParams,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let mut messages: Vec<CosmosMsg> = vec![];

    match params.incentive_asset.info.clone() {
        AssetInfo::NativeToken {
            denom: incentive_asset_denom,
        } => {
            let coin_sent = info
                .funds
                .iter()
                .find(|sent| sent.denom == incentive_asset_denom)
                .ok_or(ContractError::AssetMismatch)?;

            match incentive_creation_fee.info.clone() {
                AssetInfo::Token { .. } => {} // only fees in native tokens are supported
                AssetInfo::NativeToken {
                    denom: incentive_fee_denom,
                } => {
                    if incentive_fee_denom != incentive_asset_denom {
                        if coin_sent.amount != params.incentive_asset.amount {
                            return Err(ContractError::AssetMismatch);
                        }
                    } else {
                        if params
                            .incentive_asset
                            .amount
                            .checked_add(incentive_creation_fee.amount)?
                            != coin_sent.amount
                        {
                            return Err(ContractError::AssetMismatch);
                        }
                    }
                }
            }
        }
        AssetInfo::Token {
            contract_addr: incentive_asset_contract_addr,
        } => {
            // make sure the incentive asset has enough allowance
            let allowance: cw20::AllowanceResponse = deps.querier.query_wasm_smart(
                incentive_asset_contract_addr.clone(),
                &cw20::Cw20QueryMsg::Allowance {
                    owner: info.sender.clone().into_string(),
                    spender: env.contract.address.clone().into_string(),
                },
            )?;

            if allowance.allowance < params.incentive_asset.amount {
                return Err(ContractError::AssetMismatch);
            }

            // create the transfer message to the incentive manager
            messages.push(
                wasm_execute(
                    env.contract.address.clone().into_string(),
                    &cw20::Cw20ExecuteMsg::TransferFrom {
                        owner: info.sender.clone().into_string(),
                        recipient: env.contract.address.clone().into_string(),
                        amount: params.incentive_asset.amount,
                    },
                    vec![],
                )?
                .into(),
            );
        }
    }

    Ok(messages)
}

/// Validates the incentive epochs. Returns a tuple of (start_epoch, end_epoch) for the incentive.
pub(crate) fn validate_incentive_epochs(
    params: &IncentiveParams,
    current_epoch: u64,
    max_incentive_epoch_buffer: u64,
) -> Result<(u64, u64), ContractError> {
    // assert epoch params are correctly set
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
    if start_epoch > current_epoch + max_incentive_epoch_buffer {
        return Err(ContractError::IncentiveStartTooFar);
    }

    Ok((start_epoch, end_epoch))
}

//todo maybe move this to position helpers??
/// Validates the `unbonding_duration` specified in the position params is within the range specified
/// in the config.
pub(crate) fn validate_unbonding_duration(
    config: &Config,
    params: &PositionParams,
) -> Result<(), ContractError> {
    if params.unbonding_duration < config.min_unbonding_duration
        || params.unbonding_duration > config.max_unbonding_duration
    {
        return Err(ContractError::InvalidUnbondingDuration {
            min: config.min_unbonding_duration,
            max: config.max_unbonding_duration,
            specified: params.unbonding_duration,
        });
    }

    Ok(())
}
