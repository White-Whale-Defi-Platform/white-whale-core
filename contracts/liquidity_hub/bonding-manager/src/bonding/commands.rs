use crate::queries::MAX_PAGE_LIMIT;
use crate::state::{
    update_bond_weight, update_global_weight, BOND, CONFIG, GLOBAL, LAST_CLAIMED_EPOCH, UNBOND,
};
use crate::{helpers, ContractError};
use cosmwasm_std::{
    ensure, Addr, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Order, Response,
    StdResult, Uint128,
};
use white_whale_std::bonding_manager::Bond;
use white_whale_std::pool_network::asset;

/// Bonds the provided asset.
pub(crate) fn bond(
    mut deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    asset: Coin,
) -> Result<Response, ContractError> {
    println!("----bond----");
    helpers::validate_buckets_not_empty(&deps)?;
    //todo maybe claim for the user
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
            last_updated: current_epoch.epoch.id,
            ..Bond::default()
        });

    // update local values
    bond = update_bond_weight(&mut deps, info.sender.clone(), current_epoch.epoch.id, bond)?;
    bond.asset.amount = bond.asset.amount.checked_add(asset.amount)?;
    bond.weight = bond.weight.checked_add(asset.amount)?;

    BOND.save(deps.storage, (&info.sender, &bond.asset.denom), &bond)?;

    // update global values
    let mut global_index = GLOBAL.load(deps.storage)?;
    // include time term in the weight

    println!("bonding global_index: {:?}", global_index);

    global_index = update_global_weight(&mut deps, current_epoch.epoch.id, global_index.clone())?;

    global_index.last_weight = global_index.last_weight.checked_add(asset.amount)?;
    global_index.bonded_amount = global_index.bonded_amount.checked_add(asset.amount)?;
    global_index.bonded_assets =
        asset::aggregate_coins(&global_index.bonded_assets, &vec![asset.clone()])?;

    GLOBAL.save(deps.storage, &global_index)?;

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
                last_updated: current_epoch.epoch.id,
                created_at_epoch: current_epoch.epoch.id,
                //previous: None,
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
