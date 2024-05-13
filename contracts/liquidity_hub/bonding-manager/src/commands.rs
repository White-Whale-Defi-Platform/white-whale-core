use cosmwasm_std::{
    ensure, Addr, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Order, Response,
    StdError, StdResult, SubMsg, Uint128, Uint64,
};

use white_whale_std::bonding_manager::{Bond, GlobalIndex, RewardBucket};
use white_whale_std::epoch_manager::epoch_manager::Epoch;
use white_whale_std::pool_network::asset;

use crate::helpers::validate_growth_rate;
use crate::queries::{get_expiring_reward_bucket, query_claimable, query_weight, MAX_PAGE_LIMIT};
use crate::state::{
    update_bond_weight, update_global_weight, BOND, CONFIG, GLOBAL, LAST_CLAIMED_EPOCH,
    REWARD_BUCKETS, UNBOND,
};
use crate::{helpers, ContractError};

/// Bonds the provided asset.
pub(crate) fn bond(
    mut deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    asset: Coin,
) -> Result<Response, ContractError> {
    println!("----bond----");
    helpers::validate_buckets(&deps)?;
    helpers::validate_claimed(&deps, &info)?;
    helpers::validate_bonding_for_current_epoch(&deps)?;

    let config = CONFIG.load(deps.storage)?;
    let current_epoch: white_whale_std::epoch_manager::epoch_manager::EpochResponse =
        deps.querier.query_wasm_smart(
            config.epoch_manager_addr,
            &white_whale_std::epoch_manager::epoch_manager::QueryMsg::CurrentEpoch {},
        )?;

    let mut bond = BOND
        .key((&info.sender, &asset.denom))
        .may_load(deps.storage)?
        .unwrap_or(Bond {
            asset: Coin {
                amount: Uint128::zero(),
                ..asset.clone()
            },
            created_at_epoch: current_epoch.epoch.id,
            updated_last: current_epoch.epoch.id,
            ..Bond::default()
        });

    // update local values
    bond.asset.amount = bond.asset.amount.checked_add(asset.amount)?;
    bond.weight = bond.weight.checked_add(asset.amount)?;
    update_bond_weight(&mut deps, info.sender.clone(), current_epoch.epoch.id, bond)?;

    // update global values
    let mut global_index = GLOBAL.may_load(deps.storage)?.unwrap_or_default();
    // include time term in the weight

    global_index.weight = global_index.weight.checked_add(asset.amount)?;
    global_index.bonded_amount = global_index.bonded_amount.checked_add(asset.amount)?;
    global_index.bonded_assets =
        asset::aggregate_coins(global_index.bonded_assets, vec![asset.clone()])?;
    update_global_weight(&mut deps, current_epoch.epoch.id, global_index)?;

    Ok(Response::default().add_attributes(vec![
        ("action", "bond".to_string()),
        ("address", info.sender.to_string()),
        ("asset", asset.to_string()),
    ]))
}

/// Unbonds the provided amount of tokens
pub(crate) fn unbond(
    mut deps: DepsMut,
    info: MessageInfo,
    env: Env,
    asset: Coin,
) -> Result<Response, ContractError> {
    ensure!(
        asset.amount > Uint128::zero(),
        ContractError::InvalidUnbondingAmount
    );

    helpers::validate_claimed(&deps, &info)?;
    helpers::validate_bonding_for_current_epoch(&deps)?;
    if let Some(mut unbond) = BOND
        .key((&info.sender, &asset.denom))
        .may_load(deps.storage)?
    {
        // check if the address has enough bond
        ensure!(
            unbond.asset.amount >= asset.amount,
            ContractError::InsufficientBond
        );

        let config = CONFIG.load(deps.storage)?;
        let current_epoch: white_whale_std::epoch_manager::epoch_manager::EpochResponse =
            deps.querier.query_wasm_smart(
                config.epoch_manager_addr,
                &white_whale_std::epoch_manager::epoch_manager::QueryMsg::CurrentEpoch {},
            )?;

        // update local values, decrease the bond
        unbond = update_bond_weight(
            &mut deps,
            info.sender.clone(),
            current_epoch.epoch.id,
            unbond.clone(),
        )?;
        let weight_slash = unbond.weight * Decimal::from_ratio(asset.amount, unbond.asset.amount);
        unbond.weight = unbond.weight.saturating_sub(weight_slash);
        unbond.asset.amount = unbond.asset.amount.saturating_sub(asset.amount);

        if unbond.asset.amount.is_zero() {
            BOND.remove(deps.storage, (&info.sender, &asset.denom));
        } else {
            BOND.save(deps.storage, (&info.sender, &asset.denom), &unbond)?;
        }

        // record the unbonding
        UNBOND.save(
            deps.storage,
            (&info.sender, &asset.denom, env.block.time.nanos()),
            &Bond {
                asset: asset.clone(),
                weight: Uint128::zero(),
                updated_last: current_epoch.epoch.id,
                created_at_epoch: current_epoch.epoch.id,
            },
        )?;
        // update global values
        let mut global_index = GLOBAL.may_load(deps.storage)?.unwrap_or_default();
        global_index = update_global_weight(&mut deps, current_epoch.epoch.id, global_index)?;
        global_index.bonded_amount = global_index.bonded_amount.saturating_sub(asset.amount);
        global_index.bonded_assets =
            white_whale_std::coin::deduct_coins(global_index.bonded_assets, vec![asset.clone()])?;
        global_index.weight = global_index.weight.saturating_sub(weight_slash);

        GLOBAL.save(deps.storage, &global_index)?;

        Ok(Response::default().add_attributes(vec![
            ("action", "unbond".to_string()),
            ("address", info.sender.to_string()),
            ("asset", asset.to_string()),
        ]))
    } else {
        Err(ContractError::NothingToUnbond)
    }
}

/// Withdraws the rewards for the provided address
pub(crate) fn withdraw(
    deps: DepsMut,
    address: Addr,
    denom: String,
) -> Result<Response, ContractError> {
    let unbondings: Vec<(u64, Bond)> = UNBOND
        .prefix((&address, &denom))
        .range(deps.storage, None, None, Order::Ascending)
        .take(MAX_PAGE_LIMIT as usize)
        .collect::<StdResult<Vec<(u64, Bond)>>>()?;

    ensure!(!unbondings.is_empty(), ContractError::NothingToWithdraw);

    let config = CONFIG.load(deps.storage)?;
    let current_epoch: white_whale_std::epoch_manager::epoch_manager::EpochResponse =
        deps.querier.query_wasm_smart(
            config.epoch_manager_addr,
            &white_whale_std::epoch_manager::epoch_manager::QueryMsg::CurrentEpoch {},
        )?;

    let mut refund_amount = Uint128::zero();
    for unbonding in unbondings {
        let (ts, bond) = unbonding;
        if current_epoch.epoch.id.saturating_sub(bond.created_at_epoch) >= config.unbonding_period {
            let denom = bond.asset.denom;

            refund_amount = refund_amount.checked_add(bond.asset.amount)?;
            UNBOND.remove(deps.storage, (&address, &denom, ts));
        }
    }

    let refund_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: address.to_string(),
        amount: vec![Coin {
            denom: denom.clone(),
            amount: refund_amount,
        }],
    });

    Ok(Response::default()
        .add_message(refund_msg)
        .add_attributes(vec![
            ("action", "withdraw".to_string()),
            ("address", address.to_string()),
            ("denom", denom),
            ("refund_amount", refund_amount.to_string()),
        ]))
}

/// Updates the configuration of the contract
pub(crate) fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    epoch_manager_addr: Option<String>,
    pool_manager_addr: Option<String>,
    unbonding_period: Option<u64>,
    growth_rate: Option<Decimal>,
) -> Result<Response, ContractError> {
    // check the owner is the one who sent the message
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut config = CONFIG.load(deps.storage)?;

    if let Some(epoch_manager_addr) = epoch_manager_addr {
        config.epoch_manager_addr = deps.api.addr_validate(&epoch_manager_addr)?;
    }

    if let Some(pool_manager_addr) = pool_manager_addr {
        config.pool_manager_addr = deps.api.addr_validate(&pool_manager_addr)?;
    }

    if let Some(unbonding_period) = unbonding_period {
        config.unbonding_period = unbonding_period;
    }

    if let Some(growth_rate) = growth_rate {
        validate_growth_rate(growth_rate)?;
        config.growth_rate = growth_rate;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default().add_attributes(vec![
        ("action", "update_config".to_string()),
        ("pool_manager_addr", config.pool_manager_addr.to_string()),
        ("epoch_manager_addr", config.epoch_manager_addr.to_string()),
        ("unbonding_period", config.unbonding_period.to_string()),
        ("growth_rate", config.growth_rate.to_string()),
    ]))
}

/// Claims pending rewards for the sender.
pub fn claim(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let claimable_reward_buckets_for_user =
        query_claimable(deps.as_ref(), Some(info.sender.to_string()))?.reward_buckets;
    ensure!(
        !claimable_reward_buckets_for_user.is_empty(),
        ContractError::NothingToClaim
    );

    let mut claimable_rewards = vec![];
    let mut attributes = vec![];
    for mut reward_bucket in claimable_reward_buckets_for_user.clone() {
        let bonding_weight_response_for_epoch = query_weight(
            deps.as_ref(),
            reward_bucket.id,
            info.sender.to_string(),
            Some(reward_bucket.global_index.clone()),
        )?;

        // if the user has no share in the bucket, skip it
        if bonding_weight_response_for_epoch.share.is_zero() {
            continue;
        };

        // sanity check
        ensure!(
            bonding_weight_response_for_epoch.share <= Decimal::percent(100u64),
            ContractError::InvalidShare
        );

        for reward in reward_bucket.total.iter() {
            let user_reward = reward.amount * bonding_weight_response_for_epoch.share;

            // make sure the reward is sound
            let reward_validation: Result<(), StdError> = reward_bucket
                .available
                .iter()
                .find(|available_fee| available_fee.denom == reward.denom)
                .map(|available_fee| {
                    if user_reward > available_fee.amount {
                        attributes.push((
                            "error",
                            ContractError::InvalidReward {
                                reward: user_reward,
                                available: available_fee.amount,
                            }
                            .to_string(),
                        ));
                    }
                    Ok(())
                })
                .ok_or(StdError::generic_err("Invalid fee"))?;

            // if the reward is invalid, skip the bucket
            match reward_validation {
                Ok(_) => {}
                Err(_) => continue,
            }

            let denom = &reward.denom;
            // add the reward
            claimable_rewards = asset::aggregate_coins(
                claimable_rewards,
                vec![Coin {
                    denom: denom.to_string(),
                    amount: user_reward,
                }],
            )?;

            // modify the bucket to reflect the new available and claimed amount
            for available_fee in reward_bucket.available.iter_mut() {
                if available_fee.denom == reward.denom {
                    available_fee.amount = available_fee.amount.saturating_sub(user_reward);
                }
            }

            if reward_bucket.claimed.is_empty() {
                reward_bucket.claimed = vec![Coin {
                    denom: denom.to_string(),
                    amount: user_reward,
                }];
            } else {
                for claimed_reward in reward_bucket.claimed.iter_mut() {
                    if claimed_reward.denom == reward.denom {
                        claimed_reward.amount = claimed_reward.amount.checked_add(user_reward)?;
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

            REWARD_BUCKETS.save(deps.storage, reward_bucket.id, &reward_bucket)?;
        }
    }

    // update the last claimed epoch for the user. it's in the first bucket on the list since it's sorted
    // in descending order
    LAST_CLAIMED_EPOCH.save(
        deps.storage,
        &info.sender,
        &claimable_reward_buckets_for_user[0].id,
    )?;

    Ok(Response::default()
        .add_attributes(vec![("action", "claim".to_string())])
        .add_attributes(attributes)
        .add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: claimable_rewards,
        })))
}

pub(crate) fn fill_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    println!("----fill_rewards----");

    // Finding the most recent bucket
    let upcoming_bucket_id = match REWARD_BUCKETS
        .keys(deps.storage, None, None, Order::Descending)
        .next()
    {
        Some(bucket_id) => bucket_id?,
        None => return Err(ContractError::Unauthorized),
    };

    let config = CONFIG.load(deps.storage)?;
    let distribution_denom = config.distribution_denom.clone();

    let mut messages: Vec<CosmosMsg> = vec![];
    let mut submessages: Vec<SubMsg> = vec![];
    // swap non-whale to whale
    // Search info funds for LP tokens, LP tokens will contain LP_SYMBOL from lp_common and the string .pair.
    let mut whale = info
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
    let remanent_coins = deps
        .querier
        .query_all_balances(env.contract.address)?
        .into_iter()
        .filter(|coin| coin.denom.ne(distribution_denom.as_str()))
        .collect::<Vec<Coin>>();

    // Each of these helpers will add messages to the messages vector
    // and may increment the whale Coin above with the result of the swaps
    helpers::handle_lp_tokens(&remanent_coins, &config, &mut submessages)?;
    helpers::swap_coins_to_main_token(
        remanent_coins,
        &deps,
        config,
        &mut whale,
        &distribution_denom,
        &mut messages,
    )?;

    // Add the whale to the funds, the whale figure now should be the result
    // of all the LP token withdrawals and swaps
    // Because we are using minimum receive, it is possible the contract can accumulate micro amounts of whale if we get more than what the swap query returned
    // If this became an issue would could look at replys instead of the query
    REWARD_BUCKETS.update(deps.storage, upcoming_bucket_id, |bucket| -> StdResult<_> {
        let mut bucket = bucket.unwrap_or_default();
        bucket.available = asset::aggregate_coins(bucket.available, vec![whale.clone()])?;
        bucket.total = asset::aggregate_coins(bucket.total, vec![whale.clone()])?;
        Ok(bucket)
    })?;
    Ok(Response::default()
        .add_messages(messages)
        .add_submessages(submessages)
        .add_attributes(vec![("action", "fill_rewards".to_string())]))
}

pub(crate) fn on_epoch_created(
    deps: DepsMut,
    info: MessageInfo,
    current_epoch: Epoch,
) -> Result<Response, ContractError> {
    cw_utils::nonpayable(&info)?;

    println!("----on_epoch_created----");
    println!("EpochChangedHook: {:?}", current_epoch);
    // A new epoch has been created, update rewards bucket and forward the expiring bucket
    let config = CONFIG.load(deps.storage)?;
    ensure!(
        info.sender == config.epoch_manager_addr,
        ContractError::Unauthorized
    );

    let global = GLOBAL.may_load(deps.storage)?;
    // This happens only on the very first epoch where Global has not been initialised yet
    if global.is_none() {
        let initial_global_index = GlobalIndex {
            updated_last: current_epoch.id,
            ..Default::default()
        };
        GLOBAL.save(deps.storage, &initial_global_index)?;
        REWARD_BUCKETS.save(
            deps.storage,
            current_epoch.id,
            &RewardBucket {
                id: current_epoch.id,
                epoch_start_time: current_epoch.start_time,
                global_index: initial_global_index,
                ..RewardBucket::default()
            },
        )?;
    }

    // Update the global index epoch id field
    let mut global = GLOBAL.load(deps.storage)?;
    global.epoch_id = current_epoch.id;

    // update the global index for the current bucket, take the current snapshot of the global index
    REWARD_BUCKETS.update(
        deps.storage,
        current_epoch.id,
        |reward_bucket| -> StdResult<_> {
            let mut reward_bucket = reward_bucket.unwrap_or_default();
            reward_bucket.global_index = global;
            Ok(reward_bucket)
        },
    )?;

    // forward fees from the expiring bucket to the new one.
    let mut expiring_reward_bucket = get_expiring_reward_bucket(deps.as_ref())?;
    if let Some(expiring_bucket) = expiring_reward_bucket.as_mut() {
        // Load all the available assets from the expiring bucket
        let amount_to_be_forwarded = REWARD_BUCKETS
            .load(deps.storage, expiring_bucket.id)?
            .available;
        REWARD_BUCKETS.update(deps.storage, current_epoch.id, |bucket| -> StdResult<_> {
            let mut bucket = bucket.unwrap_or_default();
            bucket.available =
                asset::aggregate_coins(bucket.available, amount_to_be_forwarded.clone())?;
            bucket.total = asset::aggregate_coins(bucket.total, amount_to_be_forwarded)?;

            Ok(bucket)
        })?;
        // Set the available assets for the expiring bucket to an empty vec now that they have been
        // forwarded
        REWARD_BUCKETS.update(deps.storage, expiring_bucket.id, |bucket| -> StdResult<_> {
            let mut bucket = bucket.unwrap_or_default();
            bucket.available = vec![];
            Ok(bucket)
        })?;
    }

    // Create a new bucket for the rewards flowing from this time on, i.e. to be distributed in
    // the next epoch. Also, forwards the expiring bucket (only 21 bucket are live at a given moment)
    let next_epoch_id = Uint64::new(current_epoch.id)
        .checked_add(Uint64::one())?
        .u64();
    REWARD_BUCKETS.save(
        deps.storage,
        next_epoch_id,
        &RewardBucket {
            id: next_epoch_id,
            epoch_start_time: current_epoch.start_time.plus_days(1),
            // this global index is to be updated the next time this hook is called, as this future epoch
            // will become the current one
            global_index: Default::default(),
            ..RewardBucket::default()
        },
    )?;

    Ok(Response::default().add_attributes(vec![("action", "epoch_changed_hook".to_string())]))
}
