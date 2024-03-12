use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, DepsMut, Env, MessageInfo, Response, WasmMsg};
use white_whale_std::pool_network::asset::{Asset, AssetInfo, PairType};

use crate::{
    helpers::{self},
    state::{get_pair_by_identifier, COLLECTABLE_PROTOCOL_FEES},
};
use crate::{
    state::{MANAGER_CONFIG, PAIRS},
    ContractError,
};
// After writing create_pair I see this can get quite verbose so attempting to
// break it down into smaller modules which house some things like swap, liquidity etc
use cosmwasm_std::{Decimal, OverflowError, Uint128};
use cw20::Cw20ExecuteMsg;
use white_whale_std::pool_network::{
    asset::{get_total_share, MINIMUM_LIQUIDITY_AMOUNT},
    U256,
};
pub const MAX_ASSETS_PER_POOL: usize = 4;
pub const LP_SYMBOL: &str = "uLP";

/// Gets the protocol fee amount for the given asset_id
pub fn get_protocol_fee_for_asset(
    collected_protocol_fees: Vec<Asset>,
    asset_id: String,
) -> Uint128 {
    let protocol_fee_asset = collected_protocol_fees
        .iter()
        .find(|&protocol_fee_asset| protocol_fee_asset.clone().get_id() == asset_id.clone())
        .cloned();

    // get the protocol fee for the given pool_asset
    if let Some(protocol_fee_asset) = protocol_fee_asset {
        protocol_fee_asset.amount
    } else {
        Uint128::zero()
    }
}

pub fn provide_liquidity(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mut assets: Vec<Asset>,
    slippage_tolerance: Option<Decimal>,
    receiver: Option<String>,
    pair_identifier: String,
) -> Result<Response, ContractError> {
    let config = MANAGER_CONFIG.load(deps.storage)?;
    // check if the deposit feature is enabled
    if !config.feature_toggle.deposits_enabled {
        return Err(ContractError::OperationDisabled(
            "provide_liquidity".to_string(),
        ));
    }
    // Verify native assets are sent
    for asset in assets.iter() {
        asset.assert_sent_native_token_balance(&info)?;
    }
    // Get the pair by the pair_identifier
    let mut pair = get_pair_by_identifier(&deps.as_ref(), pair_identifier.clone())?;

    let mut pool_assets = pair.assets.clone();
    let mut messages: Vec<CosmosMsg> = vec![];

    for (i, pool) in assets.iter_mut().enumerate() {
        // If the pool is token contract, then we need to execute TransferFrom msg to receive funds
        if let AssetInfo::Token { contract_addr, .. } = &pool.info {
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.to_string(),
                    recipient: env.contract.address.clone().to_string(),
                    amount: pool.amount,
                })?,
                funds: vec![],
            }));
        }
        // Increment the pool asset amount by the amount sent
        pool_assets[i].amount = pool_assets[i].amount.checked_add(pool.amount).unwrap();
    }
    // After totting up the pool assets we need to check if any of them are zero
    if pool_assets.iter().any(|deposit| deposit.amount.is_zero()) {
        return Err(ContractError::InvalidZeroAmount {});
    }

    // // deduct protocol fee from pools
    // TODO: Replace with fill rewards msg
    let collected_protocol_fees = COLLECTABLE_PROTOCOL_FEES
        .load(deps.storage, &pair.liquidity_token.to_string())
        .unwrap_or_default();
    for pool in pool_assets.iter_mut() {
        let protocol_fee =
            get_protocol_fee_for_asset(collected_protocol_fees.clone(), pool.clone().get_id());
        pool.amount = pool.amount.checked_sub(protocol_fee).unwrap();
    }

    let liquidity_token = match pair.liquidity_token.clone() {
        AssetInfo::Token { contract_addr } => deps.api.addr_validate(&contract_addr)?.to_string(),
        AssetInfo::NativeToken { denom } => denom,
    };

    // Compute share and other logic based on the number of assets
    let _share = Uint128::zero();
    let total_share = get_total_share(&deps.as_ref(), liquidity_token.clone())?;

    let share = match &pair.pair_type {
        PairType::ConstantProduct => {
            if total_share == Uint128::zero() {
                // Make sure at least MINIMUM_LIQUIDITY_AMOUNT is deposited to mitigate the risk of the first
                // depositor preventing small liquidity providers from joining the pool
                let share = Uint128::new(
                    (U256::from(pool_assets[0].amount.u128())
                        .checked_mul(U256::from(pool_assets[1].amount.u128()))
                        .ok_or::<ContractError>(ContractError::LiquidityShareComputation {}))?
                    .integer_sqrt()
                    .as_u128(),
                )
                .checked_sub(MINIMUM_LIQUIDITY_AMOUNT)
                .map_err(|_| {
                    ContractError::InvalidInitialLiquidityAmount(MINIMUM_LIQUIDITY_AMOUNT)
                })?;
                // share should be above zero after subtracting the MINIMUM_LIQUIDITY_AMOUNT
                if share.is_zero() {
                    return Err(ContractError::InvalidInitialLiquidityAmount(
                        MINIMUM_LIQUIDITY_AMOUNT,
                    ));
                }

                messages.push(white_whale_std::lp_common::mint_lp_token_msg(
                    liquidity_token,
                    &info.sender,
                    &env.contract.address,
                    share,
                )?);

                share
            } else {
                let share = {
                    let numerator = U256::from(pool_assets[0].amount.u128())
                        .checked_mul(U256::from(total_share.u128()))
                        .ok_or::<ContractError>(ContractError::LiquidityShareComputation {})?;

                    let denominator = U256::from(pool_assets[0].amount.u128());

                    let result = numerator
                        .checked_div(denominator)
                        .ok_or::<ContractError>(ContractError::LiquidityShareComputation {})?;

                    Uint128::from(result.as_u128())
                };

                let amount = std::cmp::min(
                    pool_assets[0]
                        .amount
                        .multiply_ratio(total_share, pool_assets[0].amount),
                    pool_assets[1]
                        .amount
                        .multiply_ratio(total_share, pool_assets[1].amount),
                );

                let deps_as = [pool_assets[0].amount, pool_assets[1].amount];
                let pools_as = [pool_assets[0].clone(), pool_assets[1].clone()];

                // assert slippage tolerance
                helpers::assert_slippage_tolerance(
                    &slippage_tolerance,
                    &deps_as,
                    &pools_as,
                    pair.pair_type.clone(),
                    amount,
                    total_share,
                )?;

                messages.push(white_whale_std::lp_common::mint_lp_token_msg(
                    liquidity_token,
                    &info.sender,
                    &env.contract.address,
                    amount,
                )?);

                share
            }
        }
        PairType::StableSwap { amp: _ } => {
            // TODO: Handle stableswap

            Uint128::one()
        }
    };

    // mint LP token to sender
    let receiver = receiver.unwrap_or_else(|| info.sender.to_string());
    pair.assets = pool_assets.clone();
    PAIRS.save(deps.storage, pair_identifier, &pair)?;
    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("action", "provide_liquidity"),
        ("sender", info.sender.as_str()),
        ("receiver", receiver.as_str()),
        (
            "assets",
            &pool_assets
                .iter()
                .map(|asset| asset.to_string())
                .collect::<Vec<_>>()
                .join(", "),
        ),
        ("share", &share.to_string()),
    ]))
}

/// Withdraws the liquidity. The user burns the LP tokens in exchange for the tokens provided, including
/// the swap fees accrued by its share of the pool.
pub fn withdraw_liquidity(
    deps: DepsMut,
    _env: Env,
    sender: Addr,
    amount: Uint128,
    pair_identifier: String,
) -> Result<Response, ContractError> {
    let config = MANAGER_CONFIG.load(deps.storage)?;
    // check if the deposit feature is enabled
    if !config.feature_toggle.deposits_enabled {
        return Err(ContractError::OperationDisabled(
            "provide_liquidity".to_string(),
        ));
    }
    // Get the pair by the pair_identifier
    let mut pair = get_pair_by_identifier(&deps.as_ref(), pair_identifier)?;

    let liquidity_token = match pair.liquidity_token {
        AssetInfo::Token { contract_addr } => contract_addr,
        AssetInfo::NativeToken { denom } => denom,
    };
    // Get the total share of the pool
    let total_share = get_total_share(&deps.as_ref(), liquidity_token.clone())?;

    // Get the ratio of the amount to withdraw to the total share
    let share_ratio: Decimal = Decimal::from_ratio(amount, total_share);
    // Use the ratio to calculate the amount of each pool asset to refund
    let refund_assets: Result<Vec<Asset>, OverflowError> = pair
        .assets
        .iter()
        .map(|pool_asset| {
            // Calc fees and use FillRewards message
            // let protocol_fee = get_protocol_fee_for_asset(
            //     collected_protocol_fees.clone(),
            //     pool_asset.clone().get_id(),
            // );

            // subtract the protocol_fee from the amount of the pool_asset
            let refund_amount = pool_asset.amount;
            Ok(Asset {
                info: pool_asset.info.clone(),
                amount: refund_amount * share_ratio,
            })
        })
        .collect();

    let refund_assets = refund_assets?;
    let mut messages: Vec<CosmosMsg> = vec![];

    // TODO: Combine the two below for loops to improve BigO time
    // Transfer the refund assets to the sender
    for asset in refund_assets.clone() {
        messages.push(asset.clone().into_msg(sender.clone())?);
    }
    // Deduct balances on pair_info by the amount of each refund asset
    for (i, pool_asset) in pair.assets.iter_mut().enumerate() {
        pool_asset.amount = pool_asset
            .amount
            .checked_sub(refund_assets[i].amount)
            .unwrap();
    }
    // Burn the LP tokens
    messages.push(white_whale_std::lp_common::burn_lp_asset_msg(
        liquidity_token,
        sender.clone(),
        amount,
    )?);

    // update pool info
    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("action", "withdraw_liquidity"),
        ("sender", sender.as_str()),
        ("withdrawn_share", &amount.to_string()),
    ]))
}
