use cosmwasm_std::{
    ensure, from_json, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Reply, Response,
    SubMsg, Uint128,
};
use cw_utils::parse_reply_execute_data;

use white_whale_std::bonding_manager::{GlobalIndex, RewardBucket, UpcomingRewardBucket};
use white_whale_std::epoch_manager::epoch_manager::Epoch;
use white_whale_std::pool_network::asset;

use crate::helpers::{fill_upcoming_reward_bucket, get_expiring_reward_bucket};
use crate::state::{CONFIG, GLOBAL, LAST_CLAIMED_EPOCH, REWARD_BUCKETS, UPCOMING_REWARD_BUCKET};
use crate::{helpers, ContractError};

/// Handles the new epoch created by the epoch manager. It creates a new reward bucket with the
/// fees that have been accrued in the previous epoch, creates a new bucket for the upcoming rewards
/// and forwards the fees from the expiring bucket to the new one.
pub(crate) fn on_epoch_created(
    deps: DepsMut,
    info: MessageInfo,
    current_epoch: Epoch,
) -> Result<Response, ContractError> {
    cw_utils::nonpayable(&info)?;

    // A new epoch has been created, update rewards bucket and forward the expiring bucket
    let config = CONFIG.load(deps.storage)?;

    ensure!(
        info.sender == config.epoch_manager_addr,
        ContractError::Unauthorized
    );

    let mut global_index = GLOBAL.load(deps.storage).unwrap_or(
        // This happens only on the very first epoch where Global has not been initialised yet
        GlobalIndex {
            epoch_id: current_epoch.id,
            last_updated: current_epoch.id,
            ..Default::default()
        },
    );

    // Update the global index epoch id
    global_index.epoch_id = current_epoch.id;

    if global_index.bonded_amount == Uint128::zero() {
        global_index.last_updated = current_epoch.id;
    }

    GLOBAL.save(deps.storage, &global_index)?;

    // Create a new reward bucket for the current epoch with the total rewards accrued in the
    // upcoming bucket item
    let mut upcoming_bucket = UPCOMING_REWARD_BUCKET.load(deps.storage)?;

    // Remove all zero amounts from the upcoming bucket
    upcoming_bucket
        .total
        .retain(|coin| coin.amount > Uint128::zero());

    let mut new_reward_bucket = RewardBucket {
        id: current_epoch.id,
        epoch_start_time: current_epoch.start_time,
        total: upcoming_bucket.total.clone(),
        available: upcoming_bucket.total,
        claimed: vec![],
        global_index,
    };

    // forward fees from the expiring bucket to the new one.
    let mut expiring_reward_bucket = get_expiring_reward_bucket(deps.as_ref())?;
    if let Some(expiring_bucket) = expiring_reward_bucket.as_mut() {
        // Aggregate the available assets from the expiring bucket to the new reward bucket
        new_reward_bucket.available = asset::aggregate_coins(
            &new_reward_bucket.available,
            &expiring_bucket.available.clone(),
        )?;
        new_reward_bucket.total =
            asset::aggregate_coins(&new_reward_bucket.total, &expiring_bucket.available.clone())?;

        // Set the available assets for the expiring bucket to an empty vec now that they have been
        // forwarded
        expiring_bucket.available.clear();
        REWARD_BUCKETS.save(deps.storage, expiring_bucket.id, expiring_bucket)?;
    }

    // Save the new reward bucket
    REWARD_BUCKETS.save(deps.storage, current_epoch.id, &new_reward_bucket)?;
    // Reset the upcoming bucket
    UPCOMING_REWARD_BUCKET.save(deps.storage, &UpcomingRewardBucket::default())?;

    Ok(Response::default().add_attributes(vec![("action", "epoch_changed_hook".to_string())]))
}

/// Fills the upcoming rewards bucket with the upcoming fees.
pub(crate) fn fill_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let distribution_denom = config.distribution_denom.clone();

    let mut messages: Vec<CosmosMsg> = vec![];
    let mut submessages: Vec<SubMsg> = vec![];
    // swap non-whale to whale

    let mut distribution_asset_in_tx = info
        .funds
        .iter()
        .find(|coin| coin.denom.eq(distribution_denom.as_str()))
        .unwrap_or(&Coin {
            denom: distribution_denom.clone(),
            amount: Uint128::zero(),
        })
        .to_owned();

    // coins (not the distribution_denom) that are laying in the contract and have not been swapped before for lack
    // of swap routes
    let remnant_coins = deps
        .querier
        .query_all_balances(env.contract.address)?
        .into_iter()
        .filter(|coin| coin.denom.ne(distribution_denom.as_str()))
        .collect::<Vec<Coin>>();

    // Each of these helpers will add messages to the messages vector
    // and may increment the distribution_denom Coin above with the result of the swaps
    helpers::handle_lp_tokens_rewards(&deps, &remnant_coins, &config, &mut submessages)?;
    helpers::swap_coins_to_main_token(
        remnant_coins,
        &deps,
        config,
        &mut distribution_asset_in_tx,
        &distribution_denom,
        &mut messages,
    )?;

    // Add the whale to the funds, the whale figure now should be the result
    // of all the swaps
    // Because we are using minimum receive, it is possible the contract can accumulate micro amounts of whale if we get more than what the swap query returned
    // If this became an issue we could look at replies instead of the query
    // The lp_tokens being withdrawn are handled in the reply entry point
    fill_upcoming_reward_bucket(deps, distribution_asset_in_tx.clone())?;

    Ok(Response::default()
        .add_messages(messages)
        .add_submessages(submessages)
        .add_attributes(vec![("action", "fill_rewards".to_string())]))
}

/// Handles the lp withdrawal reply. It will swap the non-distribution denom coins to the
/// distribution denom and aggregate the funds to the upcoming reward bucket.
pub fn handle_lp_withdrawal_reply(deps: DepsMut, msg: Reply) -> Result<Response, ContractError> {
    // Read the coins sent via data on the withdraw response of the pool manager
    let execute_contract_response = parse_reply_execute_data(msg.clone()).unwrap();
    let data = execute_contract_response
        .data
        .ok_or(ContractError::Unauthorized)?;

    let coins: Vec<Coin> = from_json(data.as_slice())?;
    let config = CONFIG.load(deps.storage)?;
    let distribution_denom = config.distribution_denom.clone();
    let mut messages = vec![];

    // Search received coins funds for the coin that is not the distribution denom
    // This will be swapped for
    let mut distribution_asset = coins
        .iter()
        .find(|coin| coin.denom.eq(distribution_denom.as_str()))
        .unwrap_or(&Coin {
            denom: distribution_denom.clone(),
            amount: Uint128::zero(),
        })
        .to_owned();

    // Swap other coins to the distribution denom
    helpers::swap_coins_to_main_token(
        coins,
        &deps,
        config,
        &mut distribution_asset,
        &distribution_denom,
        &mut messages,
    )?;

    // update the upcoming bucket with the new funds
    fill_upcoming_reward_bucket(deps, distribution_asset.clone())?;

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("total_withdrawn", msg.id.to_string()))
}

/// Claims pending rewards for the sender.
pub fn claim(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let (total_claimable_rewards, attributes, modified_reward_buckets) =
        helpers::calculate_rewards(&deps.as_ref(), info.sender.clone(), true)?;

    // Save the modified reward buckets
    for (reward_id, claimed_rewards) in modified_reward_buckets {
        REWARD_BUCKETS.update(
            deps.storage,
            reward_id,
            |reward_bucket| -> Result<_, ContractError> {
                let mut reward_bucket = reward_bucket.unwrap();

                for reward in claimed_rewards.iter() {
                    for available_fee in reward_bucket.available.iter_mut() {
                        if available_fee.denom == reward.denom {
                            available_fee.amount =
                                available_fee.amount.saturating_sub(reward.amount);
                        }
                    }

                    reward_bucket
                        .available
                        .retain(|coin| coin.amount > Uint128::zero());

                    if reward_bucket.claimed.is_empty() {
                        reward_bucket.claimed = vec![Coin {
                            denom: reward.denom.clone(),
                            amount: reward.amount,
                        }];
                    } else {
                        for claimed_reward in reward_bucket.claimed.iter_mut() {
                            if claimed_reward.denom == reward.denom {
                                claimed_reward.amount =
                                    claimed_reward.amount.checked_add(reward.amount)?;
                            }

                            // sanity check, should never happen
                            for total_reward in reward_bucket.total.iter() {
                                if total_reward.denom == claimed_reward.denom {
                                    ensure!(
                                        claimed_reward.amount <= total_reward.amount,
                                        ContractError::InvalidShare
                                    );
                                }
                            }
                        }
                    }
                }

                Ok(reward_bucket)
            },
        )?;
    }

    // update the last claimed epoch for the user. it's in the first bucket on the list since it's sorted
    // in descending order
    let config = CONFIG.load(deps.storage)?;
    let current_epoch: white_whale_std::epoch_manager::epoch_manager::EpochResponse =
        deps.querier.query_wasm_smart(
            config.epoch_manager_addr,
            &white_whale_std::epoch_manager::epoch_manager::QueryMsg::CurrentEpoch {},
        )?;

    LAST_CLAIMED_EPOCH.save(deps.storage, &info.sender, &current_epoch.epoch.id)?;

    // Create the response based on whether there are claimable rewards to avoid sending empty coins
    let mut response = Response::default()
        .add_attributes(vec![("action", "claim".to_string())])
        .add_attributes(attributes);

    if !total_claimable_rewards.is_empty() {
        response = response.add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: total_claimable_rewards,
        }));
    }

    Ok(response)
}
