use cosmwasm_std::{
    ensure, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response,
};
use cw_utils::one_coin;

use white_whale_std::pool_network::asset::MINIMUM_LIQUIDITY_AMOUNT;
use white_whale_std::tokenfactory;

use crate::state::{get_vault_by_identifier, get_vault_by_lp, CONFIG, ONGOING_FLASHLOAN, VAULTS};
use crate::ContractError;

/// Deposits an asset into the vault
pub fn deposit(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    vault_identifier: &String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // check that deposits are enabled and if that there's a flash-loan ongoing
    if !config.deposit_enabled || ONGOING_FLASHLOAN.load(deps.storage)? {
        return Err(ContractError::Unauthorized {});
    }

    let Coin { denom, amount } = one_coin(info)?;

    let mut vault = get_vault_by_identifier(&deps.as_ref(), vault_identifier.to_owned())?;
    // check that the asset sent matches the vault
    ensure!(
        vault.asset.denom == denom,
        ContractError::AssetMismatch {
            expected: vault.asset.denom,
            actual: denom,
        }
    );

    let mut messages: Vec<CosmosMsg> = vec![];

    // mint LP token for the sender
    let total_lp_share = deps.querier.query_supply(vault.lp_denom.clone())?.amount;

    let lp_amount = if total_lp_share.is_zero() {
        // Make sure at least MINIMUM_LIQUIDITY_AMOUNT is deposited to mitigate the risk of the first
        // depositor preventing small liquidity providers from joining the vault
        let share = amount
            .checked_sub(MINIMUM_LIQUIDITY_AMOUNT)
            .map_err(|_| ContractError::InvalidInitialLiquidityAmount(MINIMUM_LIQUIDITY_AMOUNT))?;

        messages.push(white_whale_std::lp_common::mint_lp_token_msg(
            vault.lp_denom.clone(),
            &env.contract.address,
            &env.contract.address,
            MINIMUM_LIQUIDITY_AMOUNT,
        )?);

        // share should be above zero after subtracting the MINIMUM_LIQUIDITY_AMOUNT
        if share.is_zero() {
            return Err(ContractError::InvalidInitialLiquidityAmount(
                MINIMUM_LIQUIDITY_AMOUNT,
            ));
        }

        share
    } else {
        // return based on a share of the vault
        amount
            .checked_mul(total_lp_share)?
            .checked_div(vault.asset.amount)?
    };

    // mint LP token to sender
    messages.push(white_whale_std::lp_common::mint_lp_token_msg(
        vault.clone().lp_denom,
        &info.sender.clone(),
        &env.contract.address,
        lp_amount,
    )?);

    // Increase the amount of the asset in this vault
    vault.asset.amount = vault.asset.amount.checked_add(amount)?;
    VAULTS.save(deps.storage, vault_identifier.to_owned(), &vault)?;

    Ok(Response::default()
        .add_messages(messages)
        .add_attributes(vec![
            ("action", "deposit"),
            ("asset", &info.funds[0].to_string()),
        ]))
}

/// Withdraws an asset from the corresponding vault.
pub fn withdraw(deps: DepsMut, env: &Env, info: &MessageInfo) -> Result<Response, ContractError> {
    let Coin {
        denom: lp_denom,
        amount: lp_amount,
    } = one_coin(info)?;
    // check if a vault with the given lp_denom exists
    let mut vault = get_vault_by_lp(&deps.as_ref(), &lp_denom)?;

    let config = CONFIG.load(deps.storage)?;

    // check that withdrawals are enabled and if that there's a flash-loan ongoing
    if !config.withdraw_enabled || ONGOING_FLASHLOAN.load(deps.storage)? {
        return Err(ContractError::Unauthorized {});
    }

    // calculate return amount based on the share of the given vault
    let liquidity_asset = vault.lp_denom.to_string();
    let total_lp_share = deps.querier.query_supply(liquidity_asset)?.amount;
    let withdraw_amount = Decimal::from_ratio(lp_amount, total_lp_share) * vault.asset.amount;

    // sanity check
    if withdraw_amount > vault.asset.amount {
        return Err(ContractError::InsufficientAssetBalance {
            asset_balance: vault.asset.amount,
            requested_amount: withdraw_amount,
        });
    }

    // send funds to the sender and burn lp tokens
    let return_asset = Coin {
        denom: vault.clone().asset.denom,
        amount: withdraw_amount,
    };
    let messages: Vec<CosmosMsg> = vec![
        BankMsg::Send {
            to_address: info.sender.clone().into_string(),
            amount: vec![return_asset.clone()],
        }
        .into(),
        tokenfactory::burn::burn(
            env.contract.address.clone(),
            info.funds[0].clone(),
            env.contract.address.to_string(),
        ),
    ];

    // decrease the amount on the asset in this vault
    vault.asset.amount = vault.asset.amount.saturating_sub(withdraw_amount);
    VAULTS.save(deps.storage, vault.identifier.clone(), &vault)?;

    Ok(Response::default()
        .add_messages(messages)
        .add_attributes(vec![
            ("action", "withdraw"),
            ("lp_amount", &lp_amount.to_string()),
            ("return_asset", &return_asset.to_string()),
        ]))
}
