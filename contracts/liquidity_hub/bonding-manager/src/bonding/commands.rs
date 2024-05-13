use crate::queries::MAX_PAGE_LIMIT;
use crate::state::{update_bond_weight, update_global_weight, BOND, CONFIG, GLOBAL, UNBOND};
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
