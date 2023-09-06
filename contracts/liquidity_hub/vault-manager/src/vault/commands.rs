use crate::state::{COLLECTED_PROTOCOL_FEES, LOAN_COUNTER, MANAGER_CONFIG, VAULTS};
use crate::ContractError;
use cosmwasm_std::{
    coins, to_binary, BankMsg, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, Uint128,
    WasmMsg,
};
use white_whale::lp_common;
use white_whale::pool_network::asset::{
    get_total_share, Asset, AssetInfo, MINIMUM_LIQUIDITY_AMOUNT,
};
use white_whale::traits::AssetReference;
use white_whale::vault_manager::Vault;

/// Deposits an asset into the vault
pub fn deposit(
    deps: &DepsMut,
    env: &Env,
    info: &MessageInfo,
    asset: &Asset,
) -> Result<Response, ContractError> {
    let config = MANAGER_CONFIG.load(deps.storage)?;

    // check that deposits are enabled
    if !config.deposit_enabled {
        return Err(ContractError::Unauthorized {});
    }

    // check that we are not currently in a flash-loan
    if LOAN_COUNTER.load(deps.storage)? != 0 {
        // more than 0 loans is being performed currently
        return Err(ContractError::Unauthorized {});
    }

    let vault = VAULTS
        .may_load(deps.storage, asset.info.get_reference())?
        .ok_or(ContractError::NonExistentVault {})?;

    // check that user sent the assets it claims to have sent
    let sent_funds = match vault.asset_info.clone() {
        AssetInfo::NativeToken { denom } => info
            .funds
            .iter()
            .filter(|c| c.denom == denom)
            .map(|c| c.amount)
            .sum::<Uint128>(),
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

    if sent_funds != asset.amount.clone() {
        return Err(ContractError::FundsMismatch {
            sent: sent_funds,
            wanted: asset.amount,
        });
    }

    let mut messages: Vec<CosmosMsg> = vec![];
    // add cw20 transfer message if needed
    if let AssetInfo::Token { contract_addr } = vault.asset_info.clone() {
        messages.push(
            WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&cw20::Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.clone().into_string(),
                    recipient: env.contract.address.clone().into_string(),
                    amount: asset.amount.clone(),
                })?,
                funds: vec![],
            }
            .into(),
        )
    }

    // mint LP token for the sender

    let total_share = get_total_share(&deps.as_ref(), vault.lp_asset.clone().to_string())?;
    let lp_amount = if total_share.is_zero() {
        // Make sure at least MINIMUM_LIQUIDITY_AMOUNT is deposited to mitigate the risk of the first
        // depositor preventing small liquidity providers from joining the vault
        let share = asset
            .amount
            .clone()
            .checked_sub(MINIMUM_LIQUIDITY_AMOUNT)
            .map_err(|_| ContractError::InvalidInitialLiquidityAmount(MINIMUM_LIQUIDITY_AMOUNT))?;

        messages.append(&mut white_whale::lp_common::mint_lp_token_msg(
            vault.lp_asset.clone().to_string(),
            env.contract.address.to_string(),
            env.contract.address.to_string(),
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
        // If the asset is native token, the balance has already increased in the vault
        // To calculate it properly we should subtract user deposit from the vault.
        // If the asset is a cw20 token, the balance has not changed yet so we don't need to subtract it
        let deposit_amount = match vault.asset_info.clone() {
            AssetInfo::NativeToken { .. } => asset.amount.clone(),
            AssetInfo::Token { .. } => Uint128::zero(),
        };

        // return based on a share of the total vault manager
        let total_deposits = asset
            .info
            .clone()
            .query_balance(&deps.querier, deps.api, env.contract.address.clone())?
            .checked_sub(deposit_amount)?;

        asset
            .amount
            .clone()
            .checked_mul(total_share)?
            .checked_div(total_deposits)?
    };

    // mint LP token to sender
    messages.append(&mut white_whale::lp_common::mint_lp_token_msg(
        vault.lp_asset.clone().to_string(),
        info.sender.clone().into_string(),
        env.contract.address.to_string(),
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
    vault: Vault,
) -> Result<Response, ContractError> {
    let config = MANAGER_CONFIG.load(deps.storage)?;

    // check that withdrawals are enabled
    if !config.withdraw_enabled {
        return Err(ContractError::Unauthorized {});
    }

    // parse sender
    let sender = deps.api.addr_validate(&sender)?;

    // calculate the size of vault and the amount of assets to withdraw
    let collected_protocol_fees = COLLECTED_PROTOCOL_FEES.load(deps.storage)?;

    let total_asset_amount = match &vault.asset_info.clone() {
        AssetInfo::NativeToken { denom } => {
            deps.querier
                .query_balance(env.contract.address.clone(), denom)?
                .amount
        }
        AssetInfo::Token { contract_addr } => {
            let balance: cw20::BalanceResponse = deps.querier.query_wasm_smart(
                contract_addr,
                &cw20::Cw20QueryMsg::Balance {
                    address: env.contract.address.clone().into_string(),
                },
            )?;
            balance.balance
        }
    } // deduct protocol fees
    .checked_sub(collected_protocol_fees.amount)?;

    let liquidity_asset = match vault.lp_asset.clone() {
        AssetInfo::Token { contract_addr } => contract_addr,
        AssetInfo::NativeToken { denom } => denom,
    };

    let total_share = get_total_share(&deps.as_ref(), liquidity_asset.clone())?;

    let withdraw_amount = Decimal::from_ratio(lp_amount, total_share) * total_asset_amount;

    // create message to send back to user if cw20
    let messages: Vec<CosmosMsg> = vec![
        match vault.asset_info {
            AssetInfo::NativeToken { denom } => BankMsg::Send {
                to_address: sender.into_string(),
                amount: coins(withdraw_amount.u128(), denom),
            }
            .into(),
            AssetInfo::Token { contract_addr } => WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                    recipient: sender.into_string(),
                    amount: withdraw_amount,
                })?,
                funds: vec![],
            }
            .into(),
        },
        lp_common::burn_lp_asset_msg(liquidity_asset, env.contract.address.to_string(), lp_amount)?,
    ];

    Ok(Response::default()
        .add_messages(messages)
        .add_attributes(vec![
            ("method", "withdraw"),
            ("lp_amount", &lp_amount.to_string()),
            ("asset_amount", &withdraw_amount.to_string()),
        ]))
}
