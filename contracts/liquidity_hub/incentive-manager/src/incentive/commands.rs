use std::collections::HashMap;

use cosmwasm_std::{
    ensure, Addr, BankMsg, Coin, CosmosMsg, Deps, DepsMut, MessageInfo, Response, Storage, Uint128,
};

use white_whale_std::coin::aggregate_coins;
use white_whale_std::incentive_manager::{EpochId, Incentive, Position, RewardsResponse};

use crate::state::{
    get_earliest_address_lp_weight, get_incentives_by_lp_denom, get_latest_address_lp_weight,
    get_positions_by_receiver, ADDRESS_LP_WEIGHT_HISTORY, CONFIG, INCENTIVES, LAST_CLAIMED_EPOCH,
    LP_WEIGHTS_HISTORY,
};
use crate::ContractError;

//todo maybe make it claim rewards PER position, or at least per lp_denom, the way it is now it can be computationally expensive
/// Claims pending rewards for incentives where the user has LP
pub(crate) fn claim(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    cw_utils::nonpayable(&info)?;

    // check if the user has any open LP positions
    let open_positions =
        get_positions_by_receiver(deps.storage, info.sender.clone().into_string(), Some(true))?;
    ensure!(!open_positions.is_empty(), ContractError::NoOpenPositions);

    let config = CONFIG.load(deps.storage)?;
    let current_epoch = white_whale_std::epoch_manager::common::get_current_epoch(
        deps.as_ref(),
        config.epoch_manager_addr.into_string(),
    )?;

    let mut total_rewards = vec![];

    for position in open_positions {
        // calculate the rewards for the position
        let rewards_response = calculate_rewards(deps.as_ref(), position, current_epoch.id, true)?;

        match rewards_response {
            RewardsResponse::ClaimRewards {
                rewards,
                modified_incentives,
            } => {
                total_rewards.append(&mut rewards.clone());

                // update the incentives with the claimed rewards
                for (incentive_identifier, claimed_reward) in modified_incentives {
                    INCENTIVES.update(
                        deps.storage,
                        &incentive_identifier,
                        |incentive| -> Result<_, ContractError> {
                            let mut incentive = incentive.unwrap();
                            incentive.claimed_amount =
                                incentive.claimed_amount.checked_add(claimed_reward)?;
                            Ok(incentive)
                        },
                    )?;
                }
            }
            _ => return Err(ContractError::Unauthorized),
        }
    }

    // update the last claimed epoch for the user
    LAST_CLAIMED_EPOCH.save(deps.storage, &info.sender, &current_epoch.id)?;

    // sync the address lp weight history for the user
    sync_address_lp_weight_history(deps.storage, &info.sender, &current_epoch.id)?;

    Ok(Response::default()
        .add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: aggregate_coins(total_rewards)?,
        }))
        .add_attributes(vec![("action", "claim".to_string())]))
}

/// Calculates the rewards for a position
/// ### Returns
/// A [RewardsResponse] with the rewards for the position. If is_claim is true, the RewardsResponse type is
/// ClaimRewards, which contains the rewards and the modified incentives (this is to modify the
/// incentives in the claim function afterwards). If is_claim is false, the RewardsResponse only returns
/// the rewards.
pub(crate) fn calculate_rewards(
    deps: Deps,
    position: Position,
    current_epoch_id: EpochId,
    is_claim: bool,
) -> Result<RewardsResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let incentives = get_incentives_by_lp_denom(
        deps.storage,
        &position.lp_asset.denom,
        None,
        Some(config.max_concurrent_incentives),
    )?;

    let last_claimed_epoch = LAST_CLAIMED_EPOCH.may_load(deps.storage, &position.receiver)?;

    // Check if the user ever claimed before
    if let Some(last_claimed_epoch) = last_claimed_epoch {
        // if the last claimed epoch is the same as the current epoch, then there is nothing to claim
        if current_epoch_id == last_claimed_epoch {
            return if is_claim {
                Ok(RewardsResponse::ClaimRewards {
                    rewards: vec![],
                    modified_incentives: Default::default(),
                })
            } else {
                Ok(RewardsResponse::RewardsResponse { rewards: vec![] })
            };
        }
    }

    let mut rewards: Vec<Coin> = vec![];
    let mut modified_incentives: HashMap<String, Uint128> = HashMap::new();

    for incentive in incentives {
        if incentive.is_expired(current_epoch_id) {
            continue;
        }

        // compute where the user can start claiming rewards for the incentive
        let start_from_epoch = compute_start_from_epoch_for_incentive(
            deps.storage,
            &incentive,
            last_claimed_epoch,
            &position.receiver,
        )?;

        // compute the weights of the user for the epochs between start_from_epoch and current_epoch_id
        let user_weights = compute_user_weights(
            deps.storage,
            &position.receiver,
            &start_from_epoch,
            &current_epoch_id,
        )?;

        // compute the incentive emissions for the epochs between start_from_epoch and current_epoch_id
        let (incentive_emissions, until_epoch) =
            compute_incentive_emissions(&incentive, &start_from_epoch, &current_epoch_id)?;

        for epoch_id in start_from_epoch..=until_epoch {
            let user_weight = user_weights[&epoch_id];
            let total_lp_weight =
                LP_WEIGHTS_HISTORY.load(deps.storage, (&position.lp_asset.denom, epoch_id))?;

            let user_share = (user_weight, total_lp_weight);

            let reward = incentive_emissions
                .get(&epoch_id)
                .unwrap_or(&Uint128::zero())
                .to_owned()
                .checked_mul_floor(user_share)?;

            // sanity check
            ensure!(
                reward.checked_add(incentive.claimed_amount)? <= incentive.incentive_asset.amount,
                ContractError::IncentiveExhausted
            );

            rewards.push(Coin {
                denom: incentive.incentive_asset.denom.clone(),
                amount: reward,
            });

            if is_claim {
                modified_incentives.insert(incentive.identifier.clone(), reward);
            }
        }
    }

    rewards = aggregate_coins(rewards)?;

    if is_claim {
        Ok(RewardsResponse::ClaimRewards {
            rewards,
            modified_incentives,
        })
    } else {
        Ok(RewardsResponse::RewardsResponse { rewards })
    }
}

/// Computes the epoch from which the user can start claiming rewards for a given incentive
pub(crate) fn compute_start_from_epoch_for_incentive(
    storage: &dyn Storage,
    incentive: &Incentive,
    last_claimed_epoch: Option<EpochId>,
    receiver: &Addr,
) -> Result<u64, ContractError> {
    let first_claimable_epoch_for_user = if let Some(last_claimed_epoch) = last_claimed_epoch {
        // if the user has claimed before, then the next epoch is the one after the last claimed epoch
        last_claimed_epoch + 1u64
    } else {
        // if the user has never claimed before but has a weight, get the epoch at which the user
        // first had a weight in the system
        get_earliest_address_lp_weight(storage, receiver)?.0
    };

    // returns the latest epoch between the first claimable epoch for the user and the start epoch
    // of the incentive, i.e. either when the incentive starts IF the incentive starts after the
    // first claimable epoch for the user, or the first claimable epoch for the user IF the incentive
    // started before the user had a weight in the system
    Ok(incentive.start_epoch.max(first_claimable_epoch_for_user))
}

/// Computes the user weights for a given LP asset. This assumes that [compute_start_from_epoch_for_incentive]
/// was called before this function, computing the start_from_epoch for the user with either the last_claimed_epoch
/// or the first epoch the user had a weight in the system.
pub(crate) fn compute_user_weights(
    storage: &dyn Storage,
    receiver: &Addr,
    start_from_epoch: &EpochId,
    current_epoch_id: &EpochId,
) -> Result<HashMap<EpochId, Uint128>, ContractError> {
    let mut user_weights = HashMap::new();

    let mut last_weight_seen = Uint128::zero();

    // starts from start_from_epoch - 1 in case the user has a last_claimed_epoch, which means the user
    // has a weight for the last_claimed_epoch. [compute_start_from_epoch_for_incentive] would return
    // last_claimed_epoch + 1 in that case, which is correct, and if the user has not modified its
    // position, the weight will be the same for start_from_epoch as it is for last_claimed_epoch.
    for epoch_id in *start_from_epoch - 1..=*current_epoch_id {
        let weight = ADDRESS_LP_WEIGHT_HISTORY.may_load(storage, (receiver, epoch_id))?;
        if let Some(weight) = weight {
            last_weight_seen = weight;
            user_weights.insert(epoch_id, weight);
        } else {
            user_weights.insert(epoch_id, last_weight_seen);
        }
    }

    Ok(user_weights)
}

/// Computes the incentive emissions for a given incentive. Let's assume for now that the incentive
/// is expanded by a multiple of the original emission rate. todo revise this
/// ### Returns
/// A pair with the incentive emissions for each epoch between start_from_epoch and the current_epoch_id in a hashmap
/// and the last epoch for which the incentive emissions were computed
fn compute_incentive_emissions(
    incentive: &Incentive,
    start_from_epoch: &EpochId,
    current_epoch_id: &EpochId,
) -> Result<(HashMap<EpochId, Uint128>, EpochId), ContractError> {
    let mut incentive_emissions = HashMap::new();

    let until_epoch = if incentive.preliminary_end_epoch < *current_epoch_id {
        incentive.preliminary_end_epoch
    } else {
        *current_epoch_id
    };

    for epoch in *start_from_epoch..=until_epoch {
        incentive_emissions.insert(epoch, incentive.emission_rate);
    }

    Ok((incentive_emissions, until_epoch))
}

/// Syncs the address lp weight history for the given address and epoch_id, removing all the previous
/// entries as the user has already claimed those epochs, and setting the weight for the current epoch.
fn sync_address_lp_weight_history(
    storage: &mut dyn Storage,
    address: &Addr,
    current_epoch_id: &u64,
) -> Result<(), ContractError> {
    let (earliest_epoch_id, _) = get_earliest_address_lp_weight(storage, address)?;
    let (latest_epoch_id, latest_address_lp_weight) =
        get_latest_address_lp_weight(storage, address)?;

    // remove previous entries
    for epoch_id in earliest_epoch_id..=latest_epoch_id {
        ADDRESS_LP_WEIGHT_HISTORY.remove(storage, (address, epoch_id));
    }

    // save the latest weight for the current epoch
    ADDRESS_LP_WEIGHT_HISTORY.save(
        storage,
        (address, *current_epoch_id),
        &latest_address_lp_weight,
    )?;

    Ok(())
}