use cosmwasm_std::{
    ensure, to_json_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, Env, MessageInfo,
    Order, Response, StdError, StdResult, Timestamp, Uint128, Uint64, WasmMsg,
};
use white_whale_std::constants::LP_SYMBOL;
use white_whale_std::pool_manager::PairInfoResponse;
use white_whale_std::pool_network::asset;

use white_whale_std::bonding_manager::Bond;

use crate::helpers::validate_growth_rate;
use crate::queries::{query_claimable, query_weight, MAX_PAGE_LIMIT};
use crate::state::{
    update_global_weight, update_local_weight, BOND, CONFIG, EPOCHS, GLOBAL, LAST_CLAIMED_EPOCH,
    UNBOND,
};
use crate::{helpers, ContractError};

/// Bonds the provided asset.
pub(crate) fn bond(
    mut deps: DepsMut,
    timestamp: Timestamp,
    info: MessageInfo,
    _env: Env,
    asset: Coin,
) -> Result<Response, ContractError> {
    helpers::validate_funds(&deps, &info, &asset, asset.denom.clone())?;

    // helpers::validate_claimed(&deps, &info)?;

    // helpers::validate_bonding_for_current_epoch(&deps, &env)?;
    let mut bond = BOND
        .key((&info.sender, &asset.denom))
        .may_load(deps.storage)?
        .unwrap_or(Bond {
            asset: Coin {
                amount: Uint128::zero(),
                ..asset.clone()
            },
            ..Bond::default()
        });

    // update local values
    bond.asset.amount = bond.asset.amount.checked_add(asset.amount)?;
    // let new_bond_weight = get_weight(timestamp, bond.weight, asset.amount, config.growth_rate, bond.timestamp)?;
    bond.weight = bond.weight.checked_add(asset.amount)?;
    bond = update_local_weight(&mut deps, info.sender.clone(), timestamp, bond)?;
    BOND.save(deps.storage, (&info.sender, &asset.denom), &bond)?;

    // update global values
    let mut global_index = GLOBAL.may_load(deps.storage)?.unwrap_or_default();
    // global_index = update_global_weight(&mut deps, timestamp, global_index)?;
    // move into one common func TODO:
    // include time term in the weight
    global_index.weight = global_index.weight.checked_add(asset.amount)?;
    global_index.bonded_amount = global_index.bonded_amount.checked_add(asset.amount)?;
    global_index.bonded_assets =
        asset::aggregate_coins(global_index.bonded_assets, vec![asset.clone()])?;
    global_index = update_global_weight(&mut deps, timestamp, global_index)?;

    GLOBAL.save(deps.storage, &global_index)?;
    println!("Bonded asset: {:?}", global_index);

    Ok(Response::default().add_attributes(vec![
        ("action", "bond".to_string()),
        ("address", info.sender.to_string()),
        ("asset", asset.to_string()),
    ]))
}

/// Unbonds the provided amount of tokens
pub(crate) fn unbond(
    mut deps: DepsMut,
    timestamp: Timestamp,
    info: MessageInfo,
    _env: Env,
    asset: Coin,
) -> Result<Response, ContractError> {
    ensure!(
        asset.amount > Uint128::zero(),
        ContractError::InvalidUnbondingAmount {}
    );

    // helpers::validate_claimed(&deps, &info)?;
    // helpers::validate_bonding_for_current_epoch(&deps, &env)?;
    if let Some(mut unbond) = BOND
        .key((&info.sender, &asset.denom))
        .may_load(deps.storage)?
    {
        // check if the address has enough bond
        ensure!(
            unbond.asset.amount >= asset.amount,
            ContractError::InsufficientBond {}
        );

        // update local values, decrease the bond
        unbond = update_local_weight(&mut deps, info.sender.clone(), timestamp, unbond.clone())?;
        let weight_slash = unbond.weight * Decimal::from_ratio(asset.amount, unbond.asset.amount);
        unbond.weight = unbond.weight.checked_sub(weight_slash)?;
        unbond.asset.amount = unbond.asset.amount.checked_sub(asset.amount)?;

        if unbond.asset.amount.is_zero() {
            BOND.remove(deps.storage, (&info.sender, &asset.denom));
        } else {
            BOND.save(deps.storage, (&info.sender, &asset.denom), &unbond)?;
        }
        // record the unbonding
        UNBOND.save(
            deps.storage,
            (&info.sender, &asset.denom, timestamp.nanos()),
            &Bond {
                asset: asset.clone(),
                weight: Uint128::zero(),
                timestamp,
            },
        )?;
        // move this to a function to be reused
        // update global values
        let mut global_index = GLOBAL.may_load(deps.storage)?.unwrap_or_default();
        global_index = update_global_weight(&mut deps, timestamp, global_index)?;
        global_index.bonded_amount = global_index.bonded_amount.checked_sub(asset.amount)?;
        global_index.bonded_assets =
            white_whale_std::coin::deduct_coins(global_index.bonded_assets, vec![asset.clone()])?;
        global_index.weight = global_index.weight.checked_sub(weight_slash)?;

        GLOBAL.save(deps.storage, &global_index)?;

        Ok(Response::default().add_attributes(vec![
            ("action", "unbond".to_string()),
            ("address", info.sender.to_string()),
            ("asset", asset.to_string()),
        ]))
    } else {
        Err(ContractError::NothingToUnbond {})
    }
}

/// Withdraws the rewards for the provided address
pub(crate) fn withdraw(
    deps: DepsMut,
    timestamp: Timestamp,
    address: Addr,
    denom: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let unbondings: Vec<(u64, Bond)> = UNBOND
        .prefix((&address, &denom))
        .range(deps.storage, None, None, Order::Ascending)
        .take(MAX_PAGE_LIMIT as usize)
        .collect::<StdResult<Vec<(u64, Bond)>>>()?;

    let mut refund_amount = Uint128::zero();

    ensure!(!unbondings.is_empty(), ContractError::NothingToWithdraw {});

    for unbonding in unbondings {
        let (ts, bond) = unbonding;
        if timestamp.minus_nanos(config.unbonding_period.u64()) >= bond.timestamp {
            // TODO: Clean up the bond asset
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
    owner: Option<String>,
    unbonding_period: Option<Uint64>,
    growth_rate: Option<Decimal>,
) -> Result<Response, ContractError> {
    // check the owner is the one who sent the message
    let mut config = CONFIG.load(deps.storage)?;
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(owner) = owner {
        config.owner = deps.api.addr_validate(&owner)?;
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
        ("owner", config.owner.to_string()),
        ("unbonding_period", config.unbonding_period.to_string()),
        ("growth_rate", config.growth_rate.to_string()),
    ]))
}

/// Claims pending rewards for the sender.
pub fn claim(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let claimable_epochs = query_claimable(deps.as_ref(), &info.sender)?.epochs;
    ensure!(
        !claimable_epochs.is_empty(),
        ContractError::NothingToClaim {}
    );
    print!("Claimable epochs: {:?}", claimable_epochs);
    let global = GLOBAL.load(deps.storage)?;
    let mut claimable_fees = vec![];
    for mut epoch in claimable_epochs.clone() {
        let bonding_weight_response = query_weight(
            deps.as_ref(),
            env.block.time,
            info.sender.to_string(),
            Some(global.clone()),
        )?;
        println!("Bonding weight response: {:?}", bonding_weight_response);

        for fee in epoch.total.iter() {
            let reward = fee.amount * bonding_weight_response.share;

            if reward.is_zero() {
                // nothing to claim
                continue;
            }
            // make sure the reward is sound
            let _ = epoch
                .available
                .iter()
                .find(|available_fee| available_fee.denom == fee.denom)
                .map(|available_fee| {
                    if reward > available_fee.amount {
                        //todo maybe we can just skip this epoch and log something on the attributes instead
                        // of returning an error and blocking the whole operation
                        // this would "solve" the case when users unbond and then those who have not claimed
                        // past epochs won't be able to do it as their rewards exceed the available claimable fees
                        // cuz their weight increased in relation to the global weight
                        return Err(ContractError::InvalidReward {});
                    }
                    Ok(())
                })
                .ok_or_else(|| StdError::generic_err("Invalid fee"))?;
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
                    available_fee.amount = available_fee.amount.checked_sub(reward)?;
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
                }
            }

            EPOCHS.save(deps.storage, &epoch.id.to_be_bytes(), &epoch)?;
        }
    }

    // update the last claimed epoch for the user
    LAST_CLAIMED_EPOCH.save(deps.storage, &info.sender, &claimable_epochs[0].id)?;

    // Make a message to send the funds to the user
    let mut messages = vec![];
    for fee in claimable_fees {
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![fee.clone()],
        }));
    }

    Ok(Response::new()
        .add_attributes(vec![("action", "claim")])
        .add_messages(messages))
}

pub(crate) fn fill_rewards(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    {
        // Use aggregate_coins to get the total amount of new coins
        // Finding the most recent EpochID
        let most_recent_epoch_id = EPOCHS
            .keys(deps.storage, None, None, Order::Descending)
            .next()
            .unwrap()?;

        let mut messages: Vec<CosmosMsg> = vec![];
        // Verify coins are coming
        // swap non-whale to whale
        // Search info funds for LP tokens, LP tokens will contain LP_SYMBOL from lp_common and the string .pair.
        let _lp_tokens = info
            .funds
            .iter()
            .filter(|coin| coin.denom.contains(".pair.") | coin.denom.contains(LP_SYMBOL));
        // LP tokens have the format "{pair_label}.pair.{identifier}.{LP_SYMBOL}", get the identifier and not the LP SYMBOL
        // let _pair_identifier = lp_tokens
        //     .map(|coin| coin.denom.split(".pair.").collect::<Vec<&str>>()[1])
        //     .next()
        //     .unwrap();

        // // if LP Tokens ,verify and withdraw then swap to whale
        // let lp_withdrawal_msg = white_whale_std::pool_manager::ExecuteMsg::WithdrawLiquidity { pair_identifier: pair_identifier.to_string() };
        // messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        //     contract_addr: ,
        //     msg: to_json_binary(&lp_withdrawal_msg)?,
        //     funds: vec![],
        // }));

        let pool_identifier = "whale-uusdc".to_string();
        let pool_query = white_whale_std::pool_manager::QueryMsg::Pair {
            pair_identifier: pool_identifier.clone(),
        };
        let resp: PairInfoResponse = deps
            .querier
            .query_wasm_smart("contract2".to_string(), &pool_query)?;
        let mut skip_swap = false;
        // Check pair 'assets' and if either one has 0 amount then don't do swaps
        resp.pair_info.assets.iter().for_each(|asset| {
            if asset.amount.is_zero() {
                skip_swap = true;
            }
        });
        // Suggested method for swaps
        // Loop over the assets in info.funds
        // If whale is in the fund object then skip that
        // Everything else either gets swapped or if its an LP token withdrawn and then swapped
        // For each swapped coin we need to simulate swap operations and get the route from SwapRoutes
        // For each swapped coin if there is no funds found in the pool found via SwapRoutes, skip it. e.g newly made pools
        // Might need to add a reply to the contract as if doing it only in this method we can only save the simulation amount in the state
        // Alternatively we could add a reply and try to get the actual amount swapped from there.

        if !skip_swap {
            let swap_operations = vec![white_whale_std::pool_manager::SwapOperation::WhaleSwap {
                token_in_denom: info.funds[0].denom.to_string(),
                token_out_denom: "uwhale".to_string(),
                pool_identifier,
            }];
            let msg = white_whale_std::pool_manager::ExecuteMsg::ExecuteSwapOperations {
                operations: swap_operations,
                minimum_receive: None,
                to: None,
                max_spread: None,
            };
            let binary_msg = to_json_binary(&msg)?;
            let wrapped_msg = WasmMsg::Execute {
                contract_addr: "contract2".to_string(),
                msg: binary_msg,
                funds: info.funds.to_vec(),
            };
            messages.push(wrapped_msg.into());
        }
        // Note: Might need to convert back to ints and use that for ranking to get the most recent ID
        // Note: After swap,
        // TODO: Remove hardcode below after more testing
        EPOCHS.update(
            deps.storage,
            &most_recent_epoch_id,
            |bucket| -> StdResult<_> {
                let mut bucket = bucket.unwrap_or_default();
                bucket.available = asset::aggregate_coins(
                    bucket.available,
                    vec![Coin {
                        denom: "uwhale".to_string(),
                        amount: Uint128::new(1000u128),
                    }],
                )?;
                Ok(bucket)
            },
        )?;
        Ok(Response::default()
            .add_messages(messages)
            .add_attributes(vec![("action", "fill_rewards".to_string())]))
    }
}
