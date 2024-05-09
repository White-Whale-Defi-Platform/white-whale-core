use cosmwasm_std::{
    ensure, Addr, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Order, Response,
    StdError, StdResult, SubMsg, Uint128, Uint64,
};

use white_whale_std::bonding_manager::{Bond, Epoch, GlobalIndex};
use white_whale_std::pool_network::asset;

use crate::helpers::validate_growth_rate;
use crate::queries::{get_expiring_epoch, query_claimable, query_weight, MAX_PAGE_LIMIT};
use crate::state::{
    update_global_weight, update_local_weight, BOND, CONFIG, EPOCHS, GLOBAL, LAST_CLAIMED_EPOCH,
    UNBOND,
};
use crate::{helpers, ContractError};

/// Bonds the provided asset.
pub(crate) fn bond(
    mut deps: DepsMut,
    info: MessageInfo,
    env: Env,
    asset: Coin,
) -> Result<Response, ContractError> {
    println!("bonding");
    helpers::validate_epochs(&deps)?;
    helpers::validate_claimed(&deps, &info)?;
    helpers::validate_bonding_for_current_epoch(&deps, &env)?;
    println!("bonding 2");

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
            ..Bond::default()
        });

    // update local values
    bond.asset.amount = bond.asset.amount.checked_add(asset.amount)?;
    bond.weight = bond.weight.checked_add(asset.amount)?;
    bond = update_local_weight(&mut deps, info.sender.clone(), current_epoch.epoch.id, bond)?;
    BOND.save(deps.storage, (&info.sender, &asset.denom), &bond)?;

    // update global values
    let mut global_index = GLOBAL.may_load(deps.storage)?.unwrap_or_default();
    // include time term in the weight
    global_index.weight = global_index.weight.checked_add(asset.amount)?;
    global_index.bonded_amount = global_index.bonded_amount.checked_add(asset.amount)?;
    global_index.bonded_assets =
        asset::aggregate_coins(global_index.bonded_assets, vec![asset.clone()])?;
    global_index = update_global_weight(&mut deps, current_epoch.epoch.id, global_index)?;

    GLOBAL.save(deps.storage, &global_index)?;

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
    helpers::validate_bonding_for_current_epoch(&deps, &env)?;
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
        unbond = update_local_weight(
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
    let claimable_epochs_for_user =
        query_claimable(deps.as_ref(), Some(info.sender.to_string()))?.epochs;
    ensure!(
        !claimable_epochs_for_user.is_empty(),
        ContractError::NothingToClaim
    );

    let mut claimable_fees = vec![];
    let mut attributes = vec![];
    for mut epoch in claimable_epochs_for_user.clone() {
        let bonding_weight_response_for_epoch = query_weight(
            deps.as_ref(),
            epoch.id,
            info.sender.to_string(),
            Some(epoch.global_index.clone()),
        )?;

        // if the user has no share in the epoch, skip it
        if bonding_weight_response_for_epoch.share.is_zero() {
            continue;
        };

        // sanity check
        ensure!(
            bonding_weight_response_for_epoch.share <= Decimal::percent(100u64),
            ContractError::InvalidShare
        );

        for fee in epoch.total.iter() {
            let reward = fee.amount * bonding_weight_response_for_epoch.share;

            // make sure the reward is sound
            let reward_validation: Result<(), StdError> = epoch
                .available
                .iter()
                .find(|available_fee| available_fee.denom == fee.denom)
                .map(|available_fee| {
                    if reward > available_fee.amount {
                        attributes.push((
                            "error",
                            ContractError::InvalidReward {
                                reward,
                                available: available_fee.amount,
                            }
                            .to_string(),
                        ));
                    }
                    Ok(())
                })
                .ok_or(StdError::generic_err("Invalid fee"))?;

            // if the reward is invalid, skip the epoch
            match reward_validation {
                Ok(_) => {}
                Err(_) => continue,
            }

            let denom = &fee.denom;
            // add the reward to the claimable fees
            claimable_fees = asset::aggregate_coins(
                claimable_fees,
                vec![Coin {
                    denom: denom.to_string(),
                    amount: reward,
                }],
            )?;

            // modify the epoch to reflect the new available and claimed amount
            for available_fee in epoch.available.iter_mut() {
                if available_fee.denom == fee.denom {
                    available_fee.amount = available_fee.amount.saturating_sub(reward);
                }
            }

            if epoch.claimed.is_empty() {
                epoch.claimed = vec![Coin {
                    denom: denom.to_string(),
                    amount: reward,
                }];
            } else {
                for claimed_fee in epoch.claimed.iter_mut() {
                    if claimed_fee.denom == fee.denom {
                        claimed_fee.amount = claimed_fee.amount.checked_add(reward)?;
                    }

                    // sanity check, should never happen
                    for total_fee in epoch.total.iter() {
                        if total_fee.denom == claimed_fee.denom {
                            ensure!(
                                claimed_fee.amount <= total_fee.amount,
                                ContractError::InvalidShare
                            );
                        }
                    }
                }
            }

            EPOCHS.save(deps.storage, &epoch.id.to_be_bytes(), &epoch)?;
        }
    }

    // update the last claimed epoch for the user. it's the first epoch in the list since it's sorted
    // in descending order
    LAST_CLAIMED_EPOCH.save(deps.storage, &info.sender, &claimable_epochs_for_user[0].id)?;

    Ok(Response::default()
        .add_attributes(vec![("action", "claim".to_string())])
        .add_attributes(attributes)
        .add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: claimable_fees,
        })))
}

pub(crate) fn fill_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    println!(
        "EPOCHS: {:?}",
        EPOCHS
            .keys(deps.storage, None, None, Order::Descending)
            .collect::<Vec<_>>()
    );

    // Finding the most recent EpochID
    let upcoming_epoch_id = match EPOCHS
        .keys(deps.storage, None, None, Order::Descending)
        .next()
    {
        Some(epoch_id) => epoch_id?,
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

    println!("remanent_coins: {:?}", remanent_coins);
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

    println!("here");
    // Add the whale to the funds, the whale figure now should be the result
    // of all the LP token withdrawals and swaps
    // Because we are using minimum receive, it is possible the contract can accumulate micro amounts of whale if we get more than what the swap query returned
    // If this became an issue would could look at replys instead of the query
    EPOCHS.update(deps.storage, &upcoming_epoch_id, |bucket| -> StdResult<_> {
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
    current_epoch: white_whale_std::epoch_manager::epoch_manager::Epoch,
) -> Result<Response, ContractError> {
    cw_utils::nonpayable(&info)?;

    println!("EpochChangedHook: {:?}", current_epoch);
    // A new epoch has been created, update rewards bucket and forward the expiring epoch
    // Store epoch and verify the sender is the epoch manager
    let config = CONFIG.load(deps.storage)?;
    ensure!(
        info.sender == config.epoch_manager_addr,
        ContractError::Unauthorized
    );

    let global = GLOBAL.may_load(deps.storage)?;
    // This happens only on the very first epoch where Global has not been initialised yet
    if global.is_none() {
        let initial_global_index = GlobalIndex {
            last_updated: current_epoch.id,
            ..Default::default()
        };
        GLOBAL.save(deps.storage, &initial_global_index)?;
        EPOCHS.save(
            deps.storage,
            &current_epoch.id.to_be_bytes(),
            &Epoch {
                id: current_epoch.id,
                start_time: current_epoch.start_time,
                global_index: initial_global_index,
                ..Epoch::default()
            },
        )?;
    }

    let global = GLOBAL.load(deps.storage)?;

    // update the global index for the current epoch, take the current snapshot of the global index
    EPOCHS.update(
        deps.storage,
        &current_epoch.id.to_be_bytes(),
        |epoch| -> StdResult<_> {
            let mut epoch = epoch.unwrap_or_default();
            epoch.global_index = global;
            Ok(epoch)
        },
    )?;

    // todo to delete once the testing is done
    let all_epochs: Vec<Epoch> = EPOCHS
        .range(deps.storage, None, None, Order::Descending)
        .map(|item| {
            let (_, epoch) = item?;
            Ok(epoch)
        })
        .collect::<StdResult<Vec<Epoch>>>()?;

    println!("EPOCHS: {:?}", all_epochs);

    // forward fees from the expiring epoch to the new one.
    let mut expiring_epoch = get_expiring_epoch(deps.as_ref())?;
    if let Some(expiring_epoch) = expiring_epoch.as_mut() {
        // Load all the available assets from the expiring epoch
        let amount_to_be_forwarded = EPOCHS
            .load(deps.storage, &expiring_epoch.id.to_be_bytes())?
            .available;
        EPOCHS.update(
            deps.storage,
            &current_epoch.id.to_be_bytes(),
            |epoch| -> StdResult<_> {
                let mut epoch = epoch.unwrap_or_default();
                epoch.available =
                    asset::aggregate_coins(epoch.available, amount_to_be_forwarded.clone())?;
                epoch.total = asset::aggregate_coins(epoch.total, amount_to_be_forwarded)?;

                Ok(epoch)
            },
        )?;
        // Set the available assets for the expiring epoch to an empty vec now that they have been
        // forwarded
        EPOCHS.update(
            deps.storage,
            &expiring_epoch.id.to_be_bytes(),
            |epoch| -> StdResult<_> {
                let mut epoch = epoch.unwrap_or_default();
                epoch.available = vec![];
                Ok(epoch)
            },
        )?;
    }

    // Create a new bucket for the rewards flowing from this time on, i.e. to be distributed in
    // the next epoch. Also, forwards the expiring epoch (only 21 epochs are live at a given moment)
    let next_epoch_id = Uint64::new(current_epoch.id)
        .checked_add(Uint64::one())?
        .u64();
    EPOCHS.save(
        deps.storage,
        &next_epoch_id.to_be_bytes(),
        &Epoch {
            id: next_epoch_id,
            start_time: current_epoch.start_time.plus_days(1),
            // this global index is to be updated the next time this hook is called, as this future epoch
            // will become the current one
            global_index: Default::default(),
            ..Epoch::default()
        },
    )?;

    Ok(Response::default().add_attributes(vec![("action", "epoch_changed_hook".to_string())]))
}
