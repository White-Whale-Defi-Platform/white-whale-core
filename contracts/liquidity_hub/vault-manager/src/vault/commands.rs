use cosmwasm_std::{
    to_json_binary, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, Uint128, WasmMsg,
};
use cw_utils::must_pay;

use white_whale_std::pool_network::asset::{
    get_total_share, Asset, AssetInfo, MINIMUM_LIQUIDITY_AMOUNT,
};
use white_whale_std::vault_manager::Vault;

use crate::helpers::assert_asset;
use crate::state::{get_vault_by_identifier, CONFIG, ONGOING_FLASHLOAN, VAULTS};
use crate::ContractError;

/// Deposits an asset into the vault
pub fn deposit(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    asset: &Asset,
    vault_identifier: &String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // check that deposits are enabled and if that there's a flash-loan ongoing
    if !config.deposit_enabled || ONGOING_FLASHLOAN.load(deps.storage)? {
        return Err(ContractError::Unauthorized {});
    }

    let mut vault = get_vault_by_identifier(&deps.as_ref(), vault_identifier.to_owned())?;

    // check that the asset sent matches the vault
    assert_asset(&vault.asset.info, &asset.info)?;

    // check that user sent the assets it claims to have sent
    let sent_funds = match vault.asset.info.clone() {
        AssetInfo::NativeToken { denom } => must_pay(info, denom.as_str())?,
        AssetInfo::Token { contract_addr } => {
            let allowance: cw20::AllowanceResponse = deps.querier.query_wasm_smart(
                contract_addr,
                &cw20::Cw20QueryMsg::Allowance {
                    owner: info.sender.clone().into_string(),
                    spender: env.contract.address.clone().into_string(),
                },
            )?;

            allowance.allowance
        }
    };

    if sent_funds != asset.amount {
        return Err(ContractError::FundsMismatch {
            sent: sent_funds,
            wanted: asset.amount,
        });
    }

    // Increase the amount of the asset in this vault
    vault.asset.amount = vault.asset.amount.checked_add(sent_funds)?;
    VAULTS.save(deps.storage, vault_identifier.to_owned(), &vault)?;

    let mut messages: Vec<CosmosMsg> = vec![];
    // add cw20 transfer message if needed
    if let AssetInfo::Token { contract_addr } = vault.asset.info.clone() {
        messages.push(
            WasmMsg::Execute {
                contract_addr,
                msg: to_json_binary(&cw20::Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.clone().into_string(),
                    recipient: env.contract.address.clone().into_string(),
                    amount: asset.amount,
                })?,
                funds: vec![],
            }
            .into(),
        )
    }

    // mint LP token for the sender
    let total_lp_share = get_total_share(&deps.as_ref(), vault.lp_asset.to_string())?;

    // todo revise this for cw20 tokens, duplicated vaults will not have total == 0
    let lp_amount = if total_lp_share.is_zero() {
        // Make sure at least MINIMUM_LIQUIDITY_AMOUNT is deposited to mitigate the risk of the first
        // depositor preventing small liquidity providers from joining the vault
        let share = asset
            .amount
            .checked_sub(MINIMUM_LIQUIDITY_AMOUNT)
            .map_err(|_| ContractError::InvalidInitialLiquidityAmount(MINIMUM_LIQUIDITY_AMOUNT))?;

        messages.append(&mut white_whale_std::lp_common::mint_lp_token_msg(
            vault.lp_asset.to_string(),
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
        // return based on a share of the vault. We subtract the deposit amount from the vault since
        // it was added previously to the vault in this function
        let vault_total_deposits = vault.asset.amount.checked_sub(asset.amount)?;

        asset
            .amount
            .checked_mul(total_lp_share)?
            .checked_div(vault_total_deposits)?
    };

    // mint LP token to sender
    messages.append(&mut white_whale_std::lp_common::mint_lp_token_msg(
        vault.lp_asset.to_string(),
        &info.sender.clone(),
        &env.contract.address,
        lp_amount,
    )?);

    Ok(Response::default()
        .add_messages(messages)
        .add_attributes(vec![("action", "deposit"), ("asset", &asset.to_string())]))
}

/// Withdraws an asset from the given vault.
pub fn withdraw(
    deps: DepsMut,
    env: Env,
    sender: String,
    lp_amount: Uint128,
    mut vault: Vault,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // check that withdrawals are enabled and if that there's a flash-loan ongoing
    if !config.withdraw_enabled || ONGOING_FLASHLOAN.load(deps.storage)? {
        return Err(ContractError::Unauthorized {});
    }

    let sender = deps.api.addr_validate(&sender)?;

    // calculate return amount based on the share of the given vault
    let liquidity_asset = vault.lp_asset.to_string();
    let total_lp_share = get_total_share(&deps.as_ref(), liquidity_asset.clone())?;
    let withdraw_amount = Decimal::from_ratio(lp_amount, total_lp_share) * vault.asset.amount;

    // sanity check
    if withdraw_amount > vault.asset.amount {
        return Err(ContractError::InsufficientAssetBalance {
            asset_balance: vault.asset.amount,
            requested_amount: withdraw_amount,
        });
    }

    // asset to return
    let return_asset = Asset {
        info: vault.asset.info.clone(),
        amount: withdraw_amount,
    };
    let messages: Vec<CosmosMsg> = vec![
        return_asset.clone().into_msg(sender)?,
        white_whale_std::lp_common::burn_lp_asset_msg(
            liquidity_asset,
            env.contract.address,
            lp_amount,
        )?,
    ];

    // decrease the amount on the asset in this vault
    vault.asset.amount = vault.asset.amount.checked_sub(withdraw_amount)?;
    VAULTS.save(deps.storage, vault.identifier.clone(), &vault)?;

    Ok(Response::default()
        .add_messages(messages)
        .add_attributes(vec![
            ("action", "withdraw"),
            ("lp_amount", &lp_amount.to_string()),
            ("return_asset", &return_asset.to_string()),
        ]))
}
