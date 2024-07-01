use cosmwasm_std::{
    ensure, Addr, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, StdError,
    Uint128,
};

use white_whale_std::bonding_manager::{Bond, BondAction, TemporalBondAction};
use white_whale_std::pool_network::asset;

use crate::helpers::temporal_bond_action_response;
use crate::state::{
    get_bonds_by_receiver, update_bond_weight, update_global_weight, BONDS, BOND_COUNTER, CONFIG,
    GLOBAL, LAST_CLAIMED_EPOCH, MAX_LIMIT,
};
use crate::{helpers, ContractError};

/// Bonds the provided asset.
pub(crate) fn bond(
    mut deps: DepsMut,
    info: &MessageInfo,
    env: Env,
    asset: &Coin,
) -> Result<Response, ContractError> {
    helpers::validate_buckets_not_empty(&deps)?;
    let config = CONFIG.load(deps.storage)?;

    if helpers::validate_claimed(&deps, info).is_err() {
        return temporal_bond_action_response(
            &mut deps,
            env.contract.address,
            TemporalBondAction {
                sender: info.sender.clone(),
                coin: asset.clone(),
                action: BondAction::Bond,
            },
            ContractError::UnclaimedRewards,
        );
    }

    if helpers::validate_bonding_for_current_epoch(&deps, &env).is_err() {
        return temporal_bond_action_response(
            &mut deps,
            config.epoch_manager_addr,
            TemporalBondAction {
                sender: info.sender.clone(),
                coin: asset.clone(),
                action: BondAction::Bond,
            },
            ContractError::EpochNotCreatedYet,
        );
    }

    let current_epoch: white_whale_std::epoch_manager::epoch_manager::EpochResponse =
        deps.querier.query_wasm_smart(
            config.epoch_manager_addr,
            &white_whale_std::epoch_manager::epoch_manager::QueryMsg::CurrentEpoch {},
        )?;

    let bonds_by_receiver = get_bonds_by_receiver(
        deps.storage,
        info.sender.to_string(),
        Some(true),
        Some(asset.denom.clone()),
        None,
        None,
    )?;

    let mut bond = if bonds_by_receiver.is_empty() {
        // the user doesn't have any bonds of the given asset

        let bond_id =
            BOND_COUNTER.update::<_, StdError>(deps.storage, |current_id| Ok(current_id + 1u64))?;

        Bond {
            id: bond_id,
            asset: Coin {
                amount: Uint128::zero(),
                ..asset.clone()
            },
            created_at_epoch: current_epoch.epoch.id,
            last_updated: current_epoch.epoch.id,
            receiver: info.sender.clone(),
            ..Bond::default()
        }
    } else {
        // sanity check
        ensure!(
            bonds_by_receiver.len() == 1usize,
            ContractError::AssetMismatch
        );

        bonds_by_receiver[0].clone()
    };

    // update bond values
    bond = update_bond_weight(&mut deps, current_epoch.epoch.id, bond)?;
    bond.asset.amount = bond.asset.amount.checked_add(asset.amount)?;
    bond.weight = bond.weight.checked_add(asset.amount)?;

    BONDS.save(deps.storage, bond.id, &bond)?;

    // update global values
    let mut global_index = GLOBAL.load(deps.storage)?;

    global_index = update_global_weight(&mut deps, current_epoch.epoch.id, global_index.clone())?;
    global_index.last_weight = global_index.last_weight.checked_add(asset.amount)?;
    global_index.bonded_amount = global_index.bonded_amount.checked_add(asset.amount)?;
    global_index.bonded_assets =
        asset::aggregate_coins(&global_index.bonded_assets, &vec![asset.clone()])?;

    GLOBAL.save(deps.storage, &global_index)?;

    // first time the user bonds it shouldn't be able to claim rewards until the next epoch. This is
    // why we save the last claimed epoch as the current epoch.
    // In case the user has already bonded before, it won't be able to bond again without first
    // claiming the pending rewards, in which case the last claimed epoch will be updated to the
    // current epoch anyway.
    LAST_CLAIMED_EPOCH.save(deps.storage, &info.sender, &current_epoch.epoch.id)?;

    Ok(Response::default().add_attributes(vec![
        ("action", "bond".to_string()),
        ("address", info.sender.to_string()),
        ("asset", asset.to_string()),
    ]))
}

/// Unbonds the provided amount of tokens
pub(crate) fn unbond(
    mut deps: DepsMut,
    info: &MessageInfo,
    env: Env,
    asset: &Coin,
) -> Result<Response, ContractError> {
    ensure!(
        asset.amount > Uint128::zero(),
        ContractError::InvalidUnbondingAmount
    );

    let config = CONFIG.load(deps.storage)?;

    if helpers::validate_claimed(&deps, info).is_err() {
        return temporal_bond_action_response(
            &mut deps,
            env.contract.address,
            TemporalBondAction {
                sender: info.sender.clone(),
                coin: asset.clone(),
                action: BondAction::Unbond,
            },
            ContractError::UnclaimedRewards,
        );
    }

    if helpers::validate_bonding_for_current_epoch(&deps, &env).is_err() {
        return temporal_bond_action_response(
            &mut deps,
            config.epoch_manager_addr,
            TemporalBondAction {
                sender: info.sender.clone(),
                coin: asset.clone(),
                action: BondAction::Unbond,
            },
            ContractError::EpochNotCreatedYet,
        );
    }

    let bonds_by_receiver = get_bonds_by_receiver(
        deps.storage,
        info.sender.to_string(),
        Some(true),
        Some(asset.denom.clone()),
        None,
        None,
    )?;

    ensure!(
        bonds_by_receiver.len() <= 1usize,
        ContractError::AssetMismatch
    );

    if bonds_by_receiver.is_empty() {
        Err(ContractError::NothingToUnbond)
    } else {
        let mut unbond = bonds_by_receiver[0].clone();

        // check if the address has enough bond
        ensure!(
            unbond.asset.amount >= asset.amount,
            ContractError::InsufficientBond
        );

        let current_epoch: white_whale_std::epoch_manager::epoch_manager::EpochResponse =
            deps.querier.query_wasm_smart(
                config.epoch_manager_addr,
                &white_whale_std::epoch_manager::epoch_manager::QueryMsg::CurrentEpoch {},
            )?;

        // update bond values, decrease the bond
        unbond = update_bond_weight(&mut deps, current_epoch.epoch.id, unbond.clone())?;
        let weight_slash = unbond.weight * Decimal::from_ratio(asset.amount, unbond.asset.amount);
        unbond.weight = unbond.weight.saturating_sub(weight_slash);
        unbond.asset.amount = unbond.asset.amount.saturating_sub(asset.amount);

        if unbond.asset.amount.is_zero() {
            BONDS.remove(deps.storage, unbond.id)?;
        } else {
            BONDS.save(deps.storage, unbond.id, &unbond)?;
        }

        let bond_id =
            BOND_COUNTER.update::<_, StdError>(deps.storage, |current_id| Ok(current_id + 1u64))?;
        // record the unbonding
        BONDS.save(
            deps.storage,
            bond_id,
            &Bond {
                id: bond_id,
                asset: asset.clone(),
                weight: Uint128::zero(),
                last_updated: current_epoch.epoch.id,
                created_at_epoch: current_epoch.epoch.id,
                unbonded_at: Some(env.block.time.seconds()),
                receiver: info.sender.clone(),
            },
        )?;
        // update global values
        let mut global_index = GLOBAL.may_load(deps.storage)?.unwrap_or_default();
        global_index = update_global_weight(&mut deps, current_epoch.epoch.id, global_index)?;
        global_index.bonded_amount = global_index.bonded_amount.saturating_sub(asset.amount);
        global_index.bonded_assets =
            white_whale_std::coin::deduct_coins(global_index.bonded_assets, vec![asset.clone()])?;
        global_index.last_weight = global_index.last_weight.saturating_sub(weight_slash);

        GLOBAL.save(deps.storage, &global_index)?;

        Ok(Response::default().add_attributes(vec![
            ("action", "unbond".to_string()),
            ("address", info.sender.to_string()),
            ("asset", asset.to_string()),
        ]))
    }
}

/// Withdraws the unbonded asset of the given denom for the provided address
pub(crate) fn withdraw(
    deps: DepsMut,
    address: Addr,
    denom: String,
) -> Result<Response, ContractError> {
    let unbondings = get_bonds_by_receiver(
        deps.storage,
        address.to_string(),
        Some(false),
        Some(denom.clone()),
        None,
        Some(MAX_LIMIT),
    )?;

    ensure!(!unbondings.is_empty(), ContractError::NothingToWithdraw);

    let config = CONFIG.load(deps.storage)?;
    let current_epoch: white_whale_std::epoch_manager::epoch_manager::EpochResponse =
        deps.querier.query_wasm_smart(
            config.epoch_manager_addr,
            &white_whale_std::epoch_manager::epoch_manager::QueryMsg::CurrentEpoch {},
        )?;

    let mut refund_amount = Uint128::zero();
    for bond in unbondings {
        if current_epoch.epoch.id.saturating_sub(bond.created_at_epoch) >= config.unbonding_period {
            refund_amount = refund_amount.checked_add(bond.asset.amount)?;
            BONDS.remove(deps.storage, bond.id)?;
        }
    }

    ensure!(!refund_amount.is_zero(), ContractError::NothingToWithdraw);

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
