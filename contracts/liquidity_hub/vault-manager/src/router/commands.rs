use cosmwasm_std::{
    coins, ensure, to_json_binary, Addr, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo,
    Order, Response, StdError, Uint128, Uint256, WasmMsg,
};

use white_whale_std::vault_manager::{CallbackMsg, ExecuteMsg};

use crate::helpers::query_balances;
use crate::state::{get_vault_by_identifier, CONFIG, ONGOING_FLASHLOAN, TEMP_BALANCES, VAULTS};
use crate::ContractError;

/// Takes a flashloan of the specified asset and executes the payload.
pub fn flash_loan(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset: Coin,
    vault_identifier: String,
    payload: Vec<CosmosMsg>,
) -> Result<Response, ContractError> {
    // check that flash loans are enabled
    let config = CONFIG.load(deps.storage)?;
    ensure!(
        config.flash_loan_enabled && !ONGOING_FLASHLOAN.load(deps.storage)?,
        ContractError::Unauthorized {}
    );

    let vault = get_vault_by_identifier(&deps.as_ref(), vault_identifier.to_owned())?;

    // check that the asset requested matches the vault
    ensure!(
        vault.asset.denom == asset.denom,
        ContractError::AssetMismatch {
            expected: vault.asset.denom,
            actual: asset.denom,
        }
    );

    // check if the vault has enough funds
    ensure!(
        asset.amount <= vault.asset.amount,
        ContractError::InsufficientAssetBalance {
            asset_balance: vault.asset.amount,
            requested_amount: asset.amount,
        }
    );

    // toggle on the flashloan indicator
    ONGOING_FLASHLOAN.update::<_, StdError>(deps.storage, |_| Ok(true))?;

    // store balances of all assets in the contract, so that we can check that other assets were not touched during the flashloan
    let balances = query_balances(deps.as_ref(), env.contract.address.clone())?;
    for (asset_info_reference, balance) in &balances {
        TEMP_BALANCES.save(deps.storage, asset_info_reference.clone(), balance)?;
    }

    // store current balance for after trade profit check
    let old_asset_balance = *balances
        .get(&asset.denom)
        .ok_or(ContractError::NonExistentVault {})?;

    let mut messages: Vec<CosmosMsg> = vec![];

    // call payload and add after flashloan callback afterwards
    messages.extend(payload);
    messages.push(
        WasmMsg::Execute {
            contract_addr: env.contract.address.into_string(),
            msg: to_json_binary(&ExecuteMsg::Callback(CallbackMsg::AfterFlashloan {
                old_asset_balance,
                loan_asset: asset.clone(),
                vault_identifier: vault_identifier.clone(),
                sender: info.sender,
            }))?,
            funds: vec![],
        }
        .into(),
    );

    Ok(Response::default()
        .add_messages(messages)
        .add_attributes(vec![
            ("action", "flash_loan".to_string()),
            ("asset", asset.to_string()),
            ("vault_identifier", vault_identifier),
        ]))
}

/// Processes callback to this contract. Callbacks can only be done by the contract itself.
pub fn callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: CallbackMsg,
) -> Result<Response, ContractError> {
    // callback can only be called by contract
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized {});
    }

    match msg {
        CallbackMsg::AfterFlashloan {
            old_asset_balance: old_balance,
            loan_asset,
            vault_identifier,
            sender,
        } => after_flashloan(deps, env, old_balance, loan_asset, vault_identifier, sender),
    }
}

/// Completes the flashloan by paying back the outstanding loan, fees and returning profits to the
/// original sender.
pub fn after_flashloan(
    deps: DepsMut,
    env: Env,
    old_balance: Uint128,
    loan_asset: Coin,
    vault_identifier: String,
    sender: Addr,
) -> Result<Response, ContractError> {
    // get balances of all assets in the vault manager
    let new_balances = query_balances(deps.as_ref(), env.contract.address)?;

    // check that no LP assets where taken during the flashloan. When a native asset is all sent, it
    // disappears from the balance vector, thus we compare the length of the original balances
    // vector with the new balances vector
    let original_native_assets_count = TEMP_BALANCES
        .keys(deps.storage, None, None, Order::Ascending)
        .count();

    ensure!(
        original_native_assets_count <= new_balances.len(),
        ContractError::FlashLoanLoss {}
    );

    // check that all assets balances are equal or greater than before flashloan
    let any_balance_lower = new_balances
        .iter()
        .any(|(asset_info_reference, &new_balance)| {
            // get old balance for the given asset. If not found (should only happen if a vault is
            // created during the flashloan process), default to zero.
            let old_balance = TEMP_BALANCES
                .load(deps.storage, asset_info_reference.clone())
                .map_or(Uint128::zero(), |old_balance| old_balance);

            new_balance < old_balance
        });

    ensure!(!any_balance_lower, ContractError::FlashLoanLoss {});

    TEMP_BALANCES.clear(deps.storage);

    let new_asset_balance = *new_balances
        .get(&loan_asset.denom)
        .ok_or(ContractError::NonExistentVault {})?;

    // calculate the fees for executing the flashloan
    let mut vault = get_vault_by_identifier(&deps.as_ref(), vault_identifier.clone())?;

    // protocol fee goes to the bonders
    let protocol_fee = Uint128::try_from(
        vault
            .fees
            .protocol_fee
            .compute(Uint256::from(loan_asset.amount)),
    )?;

    // flashloan fee stays in the vault
    let flash_loan_fee = Uint128::try_from(
        vault
            .fees
            .flash_loan_fee
            .compute(Uint256::from(loan_asset.amount)),
    )?;

    // check that new balance is greater than expected
    let required_amount = old_balance
        .checked_add(protocol_fee)?
        .checked_add(flash_loan_fee)?;

    ensure!(
        required_amount <= new_asset_balance,
        ContractError::NegativeProfit {
            old_balance,
            current_balance: new_asset_balance,
            required_amount,
        }
    );

    // add the flashloan fee to the vault
    vault.asset.amount = vault.asset.amount.checked_add(flash_loan_fee)?;
    VAULTS.save(deps.storage, vault_identifier, &vault)?;

    // calculate flashloan profit
    let profit = new_asset_balance
        .checked_sub(old_balance)?
        .saturating_sub(protocol_fee)
        .saturating_sub(flash_loan_fee);

    let mut messages: Vec<CosmosMsg> = vec![];

    if !profit.is_zero() {
        // send profit to sender
        messages.push(
            BankMsg::Send {
                to_address: sender.to_string(),
                amount: coins(profit.u128(), loan_asset.denom.clone()),
            }
            .into(),
        );
    }

    let config = CONFIG.load(deps.storage)?;

    // send protocol fee to whale lair
    messages.push(white_whale_std::bonding_manager::fill_rewards_msg(
        config.whale_lair_addr.into_string(),
        coins(protocol_fee.u128(), loan_asset.denom),
    )?);

    // toggle off ongoing flashloan flag
    ONGOING_FLASHLOAN.update::<_, StdError>(deps.storage, |_| Ok(false))?;

    Ok(Response::default()
        .add_messages(messages)
        .add_attributes(vec![
            ("action", "after_flashloan".to_string()),
            ("vault_identifier", vault.identifier),
            ("profit", profit.to_string()),
            ("protocol_fee", protocol_fee.to_string()),
            ("flash_loan_fee", flash_loan_fee.to_string()),
        ]))
}
