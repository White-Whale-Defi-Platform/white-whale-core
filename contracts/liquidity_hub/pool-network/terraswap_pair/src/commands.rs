#[cfg(feature = "osmosis")]
use anybuf::Anybuf;
#[cfg(any(feature = "osmosis_token_factory", feature = "injective"))]
use cosmwasm_std::coins;
use cosmwasm_std::{
    from_json, to_json_binary, Addr, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, OverflowError,
    Response, StdError, StdResult, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

#[cfg(any(feature = "osmosis_token_factory", feature = "injective"))]
use white_whale_std::pool_network::asset::is_factory_token;
use white_whale_std::pool_network::asset::{
    get_total_share, Asset, AssetInfo, AssetInfoRaw, PairInfoRaw, PairType,
    MINIMUM_LIQUIDITY_AMOUNT,
};
#[cfg(feature = "injective")]
use white_whale_std::pool_network::denom_injective::{Coin, MsgBurn, MsgMint};
#[cfg(feature = "osmosis_token_factory")]
use white_whale_std::pool_network::denom_osmosis::{Coin, MsgBurn, MsgMint};
use white_whale_std::pool_network::pair::{Config, Cw20HookMsg, FeatureToggle, PoolFee};
use white_whale_std::pool_network::{swap, U256};

use crate::error::ContractError;
use crate::helpers;
use crate::helpers::{
    compute_d, compute_lp_mint_amount_for_stableswap_deposit, get_protocol_fee_for_asset,
};
use crate::state::{
    store_fee, ALL_TIME_BURNED_FEES, ALL_TIME_COLLECTED_PROTOCOL_FEES, COLLECTED_PROTOCOL_FEES,
    CONFIG, PAIR_INFO,
};

const MINIMUM_COLLECTABLE_BALANCE: Uint128 = Uint128::new(1_000u128);

/// Receives cw20 tokens. Used to swap and withdraw from the pool.
/// If the Cw20HookMsg is Swap, the user must call IncreaseAllowance on the cw20 token first to allow
/// the contract to spend the tokens and perform the swap operation.
pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let contract_addr = info.sender.clone();
    let feature_toggle: FeatureToggle = CONFIG.load(deps.storage)?.feature_toggle;

    match from_json(&cw20_msg.msg) {
        Ok(Cw20HookMsg::Swap {
            belief_price,
            max_spread,
            to,
        }) => {
            // check if the swap feature is enabled
            if !feature_toggle.swaps_enabled {
                return Err(ContractError::OperationDisabled("swap".to_string()));
            }

            // only asset contract can execute this message
            let mut authorized: bool = false;
            let config: PairInfoRaw = PAIR_INFO.load(deps.storage)?;
            let pools: [Asset; 2] =
                config.query_pools(&deps.querier, deps.api, env.contract.address.clone())?;
            for pool in pools.iter() {
                if let AssetInfo::Token { contract_addr, .. } = &pool.info {
                    if contract_addr == &info.sender {
                        authorized = true;
                    }
                }
            }

            if !authorized {
                return Err(ContractError::Unauthorized {});
            }

            let to_addr = if let Some(to_addr) = to {
                Some(deps.api.addr_validate(to_addr.as_str())?)
            } else {
                None
            };

            swap(
                deps,
                env,
                info,
                Addr::unchecked(cw20_msg.sender),
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: contract_addr.to_string(),
                    },
                    amount: cw20_msg.amount,
                },
                belief_price,
                max_spread,
                to_addr,
            )
        }
        Ok(Cw20HookMsg::WithdrawLiquidity {}) => {
            // check if the withdrawal feature is enabled
            if !feature_toggle.withdrawals_enabled {
                return Err(ContractError::OperationDisabled(
                    "withdraw_liquidity".to_string(),
                ));
            }

            let config: PairInfoRaw = PAIR_INFO.load(deps.storage)?;
            let cw20_lp_token = match config.liquidity_token {
                AssetInfoRaw::Token { contract_addr } => contract_addr,
                AssetInfoRaw::NativeToken { .. } => return Err(ContractError::Unauthorized {}),
            };

            if deps.api.addr_canonicalize(info.sender.as_str())? != cw20_lp_token {
                return Err(ContractError::Unauthorized {});
            }

            let sender_addr = deps.api.addr_validate(cw20_msg.sender.as_str())?;
            withdraw_liquidity(deps, env, sender_addr, cw20_msg.amount)
        }
        Err(err) => Err(ContractError::Std(err)),
    }
}

/// Provides liquidity. The user must IncreaseAllowance on the token when providing cw20 tokens
pub fn provide_liquidity(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    assets: [Asset; 2],
    slippage_tolerance: Option<Decimal>,
    receiver: Option<String>,
) -> Result<Response, ContractError> {
    // check if the deposit feature is enabled
    let feature_toggle: FeatureToggle = CONFIG.load(deps.storage)?.feature_toggle;
    if !feature_toggle.deposits_enabled {
        return Err(ContractError::OperationDisabled(
            "provide_liquidity".to_string(),
        ));
    }

    for asset in assets.iter() {
        asset.assert_sent_native_token_balance(&info)?;
    }

    let pair_info: PairInfoRaw = PAIR_INFO.load(deps.storage)?;
    let mut pools: [Asset; 2] =
        pair_info.query_pools(&deps.querier, deps.api, env.contract.address.clone())?;
    let deposits: [Uint128; 2] = [
        assets
            .iter()
            .find(|a| a.info.equal(&pools[0].info))
            .map(|a| a.amount)
            .expect("Wrong asset info is given"),
        assets
            .iter()
            .find(|a| a.info.equal(&pools[1].info))
            .map(|a| a.amount)
            .expect("Wrong asset info is given"),
    ];

    if deposits[0].is_zero() || deposits[1].is_zero() {
        return Err(ContractError::InvalidZeroAmount {});
    }

    let mut messages: Vec<CosmosMsg> = vec![];
    for (i, pool) in pools.iter_mut().enumerate() {
        // If the pool is token contract, then we need to execute TransferFrom msg to receive funds
        if let AssetInfo::Token { contract_addr, .. } = &pool.info {
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.to_string(),
                    recipient: env.contract.address.to_string(),
                    amount: deposits[i],
                })?,
                funds: vec![],
            }));
        } else {
            // If the asset is native token, balance is already increased
            // To calculate it properly we should subtract user deposit from the pool
            pool.amount = pool.amount.checked_sub(deposits[i])?;
        }
    }

    // deduct protocol fee from pools
    let collected_protocol_fees = COLLECTED_PROTOCOL_FEES.load(deps.storage)?;
    for pool in pools.iter_mut() {
        let protocol_fee =
            get_protocol_fee_for_asset(collected_protocol_fees.clone(), pool.clone().get_id());
        pool.amount = pool.amount.checked_sub(protocol_fee)?;
    }

    let liquidity_token = match pair_info.liquidity_token {
        AssetInfoRaw::Token { contract_addr } => {
            deps.api.addr_humanize(&contract_addr)?.to_string()
        }
        AssetInfoRaw::NativeToken { denom } => denom,
    };

    let total_share = get_total_share(&deps.as_ref(), liquidity_token.clone())?;

    let share = match &pair_info.pair_type {
        PairType::StableSwap { amp } => {
            if total_share == Uint128::zero() {
                // Make sure at least MINIMUM_LIQUIDITY_AMOUNT is deposited to mitigate the risk of the first
                // depositor preventing small liquidity providers from joining the pool
                let min_lp_token_amount = MINIMUM_LIQUIDITY_AMOUNT * Uint128::from(2u8);

                let share = Uint128::try_from(compute_d(amp, deposits[0], deposits[1]).unwrap())?
                    .saturating_sub(min_lp_token_amount);

                messages.append(&mut mint_lp_token_msg(
                    liquidity_token.clone(),
                    env.contract.address.to_string(),
                    env.contract.address.to_string(),
                    min_lp_token_amount,
                )?);

                // share should be above zero after subtracting the min_lp_token_amount
                if share.is_zero() {
                    return Err(ContractError::InvalidInitialLiquidityAmount(
                        min_lp_token_amount,
                    ));
                }

                share
            } else {
                let amount = compute_lp_mint_amount_for_stableswap_deposit(
                    amp,
                    deposits[0],
                    deposits[1],
                    pools[0].amount,
                    pools[1].amount,
                    total_share,
                )
                .unwrap();

                // assert slippage tolerance
                helpers::assert_slippage_tolerance(
                    &slippage_tolerance,
                    &deposits,
                    &pools,
                    pair_info.pair_type,
                    amount,
                    total_share,
                )?;

                amount
            }
        }
        PairType::ConstantProduct => {
            if total_share == Uint128::zero() {
                // Make sure at least MINIMUM_LIQUIDITY_AMOUNT is deposited to mitigate the risk of the first
                // depositor preventing small liquidity providers from joining the pool
                let share = Uint128::new(
                    (U256::from(deposits[0].u128())
                        .checked_mul(U256::from(deposits[1].u128()))
                        .ok_or::<ContractError>(ContractError::LiquidityShareComputation {}))?
                    .integer_sqrt()
                    .as_u128(),
                )
                .checked_sub(MINIMUM_LIQUIDITY_AMOUNT)
                .map_err(|_| {
                    ContractError::InvalidInitialLiquidityAmount(MINIMUM_LIQUIDITY_AMOUNT)
                })?;

                messages.append(&mut mint_lp_token_msg(
                    liquidity_token.clone(),
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
                // min(1, 2)
                // 1. sqrt(deposit_0 * exchange_rate_0_to_1 * deposit_0) * (total_share / sqrt(pool_0 * pool_1))
                // == deposit_0 * total_share / pool_0
                // 2. sqrt(deposit_1 * exchange_rate_1_to_0 * deposit_1) * (total_share / sqrt(pool_1 * pool_1))
                // == deposit_1 * total_share / pool_1
                //todo fix the index stuff here
                let amount = std::cmp::min(
                    deposits[0].multiply_ratio(total_share, pools[0].amount),
                    deposits[1].multiply_ratio(total_share, pools[1].amount),
                );

                // assert slippage tolerance
                helpers::assert_slippage_tolerance(
                    &slippage_tolerance,
                    &deposits,
                    &pools,
                    pair_info.pair_type,
                    amount,
                    total_share,
                )?;

                amount
            }
        }
    };

    // mint LP token to sender
    let receiver = receiver.unwrap_or_else(|| info.sender.to_string());
    messages.append(&mut mint_lp_token_msg(
        liquidity_token,
        receiver.clone(),
        env.contract.address.to_string(),
        share,
    )?);

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("action", "provide_liquidity"),
        ("sender", info.sender.as_str()),
        ("receiver", receiver.as_str()),
        ("assets", &format!("{}, {}", assets[0], assets[1])),
        ("share", &share.to_string()),
    ]))
}

/// Withdraws the liquidity. The user burns the LP tokens in exchange for the tokens provided, including
/// the swap fees accrued by its share of the pool.
pub fn withdraw_liquidity(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let pair_info: PairInfoRaw = PAIR_INFO.load(deps.storage)?;
    let pool_assets: [Asset; 2] =
        pair_info.query_pools(&deps.querier, deps.api, env.contract.address.clone())?;

    let liquidity_token = match pair_info.liquidity_token {
        AssetInfoRaw::Token { contract_addr } => {
            deps.api.addr_humanize(&contract_addr)?.to_string()
        }
        AssetInfoRaw::NativeToken { denom } => denom,
    };

    let total_share = get_total_share(&deps.as_ref(), liquidity_token.clone())?;

    let collected_protocol_fees = COLLECTED_PROTOCOL_FEES.load(deps.storage)?;

    let share_ratio: Decimal = Decimal::from_ratio(amount, total_share);

    let refund_assets: Result<Vec<Asset>, OverflowError> = pool_assets
        .iter()
        .map(|pool_asset| {
            let protocol_fee = get_protocol_fee_for_asset(
                collected_protocol_fees.clone(),
                pool_asset.clone().get_id(),
            );

            // subtract the protocol_fee from the amount of the pool_asset
            let refund_amount = pool_asset.amount.checked_sub(protocol_fee)?;
            Ok(Asset {
                info: pool_asset.info.clone(),
                amount: refund_amount * share_ratio,
            })
        })
        .collect();

    let refund_assets = refund_assets?;

    let burn_lp_token_msg =
        burn_lp_token_msg(liquidity_token, env.contract.address.to_string(), amount)?;

    // update pool info
    Ok(Response::new()
        .add_messages(vec![
            refund_assets[0].clone().into_msg(sender.clone())?,
            refund_assets[1].clone().into_msg(sender.clone())?,
            // burn liquidity token
            burn_lp_token_msg,
        ])
        .add_attributes(vec![
            ("action", "withdraw_liquidity"),
            ("sender", sender.as_str()),
            ("withdrawn_share", &amount.to_string()),
            (
                "refund_assets",
                &format!("{}, {}", refund_assets[0], refund_assets[1]),
            ),
        ]))
}

/// Swaps tokens from the pool. The user provides an offer asset and receives the ask asset in return.
#[allow(clippy::too_many_arguments)]
pub fn swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sender: Addr,
    offer_asset: Asset,
    belief_price: Option<Decimal>,
    max_spread: Option<Decimal>,
    to: Option<Addr>,
) -> Result<Response, ContractError> {
    offer_asset.assert_sent_native_token_balance(&info)?;

    let pair_info = PAIR_INFO.load(deps.storage)?;

    // determine what's the offer and ask pool based on the offer_asset
    let offer_pool: Asset;
    let ask_pool: Asset;
    let collected_protocol_fees = COLLECTED_PROTOCOL_FEES.load(deps.storage)?;

    let offer_decimal: u8;
    let ask_decimal: u8;

    // To calculate pool amounts properly we should subtract user deposit and the protocol fees from the pool
    #[cfg(not(feature = "osmosis"))]
    let contract_addr = env.contract.address;
    #[cfg(feature = "osmosis")]
    let contract_addr = env.contract.address.clone();

    let pools = pair_info
        .query_pools(&deps.querier, deps.api, contract_addr)?
        .into_iter()
        .map(|mut pool| {
            // subtract the protocol fee from the pool
            let protocol_fee =
                get_protocol_fee_for_asset(collected_protocol_fees.clone(), pool.clone().get_id());
            pool.amount = pool.amount.checked_sub(protocol_fee)?;

            if pool.info.equal(&offer_asset.info) {
                pool.amount = pool.amount.checked_sub(offer_asset.amount)?
            }

            Ok(pool)
        })
        .collect::<StdResult<Vec<_>>>()?;

    if offer_asset.info.equal(&pools[0].info) {
        offer_pool = pools[0].clone();
        ask_pool = pools[1].clone();

        offer_decimal = pair_info.asset_decimals[0];
        ask_decimal = pair_info.asset_decimals[1];
    } else if offer_asset.info.equal(&pools[1].info) {
        offer_pool = pools[1].clone();
        ask_pool = pools[0].clone();

        offer_decimal = pair_info.asset_decimals[1];
        ask_decimal = pair_info.asset_decimals[0];
    } else {
        return Err(ContractError::AssetMismatch {});
    }

    let offer_amount = offer_asset.amount;
    let config = CONFIG.load(deps.storage)?;

    let swap_computation = helpers::compute_swap(
        offer_pool.amount,
        ask_pool.amount,
        offer_amount,
        config.pool_fees,
        &pair_info.pair_type,
        offer_decimal,
        ask_decimal,
    )?;

    let return_asset = Asset {
        info: ask_pool.info.clone(),
        amount: swap_computation.return_amount,
    };

    let fees = {
        let base_fees = swap_computation
            .swap_fee_amount
            .checked_add(swap_computation.protocol_fee_amount)?
            .checked_add(swap_computation.burn_fee_amount)?;

        #[cfg(feature = "osmosis")]
        {
            base_fees.checked_add(swap_computation.osmosis_fee_amount)?
        }

        #[cfg(not(feature = "osmosis"))]
        {
            base_fees
        }
    };

    // check max spread limit if exist
    swap::assert_max_spread(
        belief_price,
        max_spread,
        offer_asset.amount,
        return_asset.amount.checked_add(fees)?,
        swap_computation.spread_amount,
    )?;

    let receiver = to.unwrap_or_else(|| sender.clone());

    let mut messages: Vec<CosmosMsg> = vec![];
    if !swap_computation.return_amount.is_zero() {
        messages.push(return_asset.into_msg(receiver.clone())?);
    }

    // burn ask_asset from the pool
    if !swap_computation.burn_fee_amount.is_zero() {
        let burn_asset = Asset {
            info: ask_pool.info.clone(),
            amount: swap_computation.burn_fee_amount,
        };

        store_fee(
            deps.storage,
            burn_asset.amount,
            burn_asset.clone().get_id(),
            ALL_TIME_BURNED_FEES,
        )?;

        messages.push(burn_asset.into_burn_msg()?);
    }

    #[cfg(feature = "osmosis")]
    if !swap_computation.osmosis_fee_amount.is_zero()
        && info.sender != config.cosmwasm_pool_interface
    {
        // send osmosis fee to the Community Pool if the swap was not initiated by the osmosis pool manager via the
        // cosmwasm pool interface
        let denom = match ask_pool.info.clone() {
            AssetInfo::Token { .. } => return Err(StdError::generic_err("Not supported").into()),
            AssetInfo::NativeToken { denom } => denom,
        };

        //https://docs.cosmos.network/v0.45/core/proto-docs.html#cosmos.distribution.v1beta1.MsgFundCommunityPool
        let community_pool_msg = CosmosMsg::Stargate {
            type_url: "/cosmos.distribution.v1beta1.MsgFundCommunityPool".to_string(),
            value: Anybuf::new()
                .append_repeated_message(
                    1,
                    &[&Anybuf::new()
                        .append_string(1, denom)
                        .append_string(2, swap_computation.osmosis_fee_amount.to_string())],
                )
                .append_string(2, &env.contract.address)
                .into_vec()
                .into(),
        };

        messages.push(community_pool_msg);
    }

    // Store the protocol fees generated by this swap. The protocol fees are collected on the ask
    // asset as shown in [compute_swap]
    store_fee(
        deps.storage,
        swap_computation.protocol_fee_amount,
        ask_pool.clone().get_id(),
        COLLECTED_PROTOCOL_FEES,
    )?;
    store_fee(
        deps.storage,
        swap_computation.protocol_fee_amount,
        ask_pool.clone().get_id(),
        ALL_TIME_COLLECTED_PROTOCOL_FEES,
    )?;

    // 1. send collateral token from the contract to a user
    // 2. stores the protocol fees
    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("action", "swap"),
        ("sender", sender.as_str()),
        ("receiver", receiver.as_str()),
        ("offer_asset", &offer_asset.info.to_string()),
        ("ask_asset", &ask_pool.info.to_string()),
        ("offer_amount", &offer_amount.to_string()),
        ("return_amount", &swap_computation.return_amount.to_string()),
        ("spread_amount", &swap_computation.spread_amount.to_string()),
        (
            "swap_fee_amount",
            &swap_computation.swap_fee_amount.to_string(),
        ),
        (
            "protocol_fee_amount",
            &swap_computation.protocol_fee_amount.to_string(),
        ),
        (
            "burn_fee_amount",
            &swap_computation.burn_fee_amount.to_string(),
        ),
        #[cfg(feature = "osmosis")]
        (
            "osmosis_fee_amount",
            &swap_computation.osmosis_fee_amount.to_string(),
        ),
        ("swap_type", pair_info.pair_type.get_label()),
    ]))
}

#[allow(unused_variables)]
/// Updates the [Config] of the contract. Only the owner of the contract can do this.
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<String>,
    fee_collector_addr: Option<String>,
    pool_fees: Option<PoolFee>,
    feature_toggle: Option<FeatureToggle>,
    cosmwasm_pool_interface: Option<String>,
) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;
    if deps.api.addr_validate(info.sender.as_str())? != config.owner {
        return Err(ContractError::Std(StdError::generic_err("unauthorized")));
    }

    if let Some(owner) = owner {
        // validate address format
        let _ = deps.api.addr_validate(&owner)?;
        config.owner = deps.api.addr_validate(&owner)?;
    }

    if let Some(pool_fees) = pool_fees {
        pool_fees.is_valid()?;
        config.pool_fees = pool_fees;
    }

    if let Some(feature_toggle) = feature_toggle {
        config.feature_toggle = feature_toggle;
    }

    if let Some(fee_collector_addr) = fee_collector_addr {
        config.fee_collector_addr = deps.api.addr_validate(fee_collector_addr.as_str())?;
    }

    #[cfg(feature = "osmosis")]
    if let Some(cosmwasm_pool_interface) = cosmwasm_pool_interface {
        config.cosmwasm_pool_interface =
            deps.api.addr_validate(cosmwasm_pool_interface.as_str())?;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

/// Collects all protocol fees accrued by the pool
pub fn collect_protocol_fees(deps: DepsMut) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // get the collected protocol fees so far
    let protocol_fees = COLLECTED_PROTOCOL_FEES.load(deps.storage)?;
    // reset the collected protocol fees
    COLLECTED_PROTOCOL_FEES.save(
        deps.storage,
        &vec![
            Asset {
                info: protocol_fees[0].clone().info,
                amount: Uint128::zero(),
            },
            Asset {
                info: protocol_fees[1].clone().info,
                amount: Uint128::zero(),
            },
        ],
    )?;

    let mut messages: Vec<CosmosMsg> = Vec::new();
    for protocol_fee in protocol_fees {
        // prevents sending protocol fees if the amount is less than the minimum collectable balance
        if protocol_fee.amount > MINIMUM_COLLECTABLE_BALANCE {
            messages.push(protocol_fee.into_msg(config.fee_collector_addr.clone())?);
        }
    }

    Ok(Response::default()
        .add_attribute("action", "collect_protocol_fees")
        .add_messages(messages))
}

/// Creates the Mint LP message
#[allow(unused_variables)]
fn mint_lp_token_msg(
    liquidity_token: String,
    recipient: String,
    sender: String,
    amount: Uint128,
) -> Result<Vec<CosmosMsg>, ContractError> {
    #[cfg(any(feature = "osmosis_token_factory", feature = "injective"))]
    if is_factory_token(liquidity_token.as_str()) {
        let mut messages = vec![];
        messages.push(<MsgMint as Into<CosmosMsg>>::into(MsgMint {
            sender: sender.clone(),
            amount: Some(Coin {
                denom: liquidity_token.clone(),
                amount: amount.to_string(),
            }),
        }));

        if sender != recipient {
            messages.push(CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
                to_address: recipient,
                amount: coins(amount.u128(), liquidity_token.as_str()),
            }));
        }

        Ok(messages)
    } else {
        Ok(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: liquidity_token,
            msg: to_json_binary(&Cw20ExecuteMsg::Mint { recipient, amount })?,
            funds: vec![],
        })])
    }

    #[cfg(all(not(feature = "osmosis_token_factory"), not(feature = "injective")))]
    Ok(vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: liquidity_token,
        msg: to_json_binary(&Cw20ExecuteMsg::Mint { recipient, amount })?,
        funds: vec![],
    })])
}

/// Creates the Burn LP message
#[allow(unused_variables)]
fn burn_lp_token_msg(
    liquidity_token: String,
    sender: String,
    amount: Uint128,
) -> Result<CosmosMsg, ContractError> {
    #[cfg(any(feature = "osmosis_token_factory", feature = "injective"))]
    if is_factory_token(liquidity_token.as_str()) {
        Ok(<MsgBurn as Into<CosmosMsg>>::into(MsgBurn {
            sender,
            amount: Some(Coin {
                denom: liquidity_token,
                amount: amount.to_string(),
            }),
        }))
    } else {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: liquidity_token,
            msg: to_json_binary(&Cw20ExecuteMsg::Burn { amount })?,
            funds: vec![],
        }))
    }
    #[cfg(all(not(feature = "osmosis_token_factory"), not(feature = "injective")))]
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: liquidity_token,
        msg: to_json_binary(&Cw20ExecuteMsg::Burn { amount })?,
        funds: vec![],
    }))
}
