use classic_bindings::TerraQuery;
use cosmwasm_std::{to_json_binary, DepsMut, Env, Reply, Response, WasmMsg};

use white_whale_std::pool_network::{
    asset::Asset, asset::AssetInfo, frontend_helper::TempState, incentive::QueryPosition,
};

use crate::{
    error::ContractError,
    state::{CONFIG, TEMP_STATE},
};

/// The reply ID for submessages after depositing to the pair contract.
pub const DEPOSIT_PAIR_REPLY_ID: u64 = 1;

/// Triggered after a new deposit is made to a pair.
///
/// Triggered to allow us to register the new contract in state.
pub fn deposit_pair(
    deps: DepsMut<TerraQuery>,
    env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    msg.result
        .into_result()
        .map_err(|e| ContractError::DepositCallback { reason: e })?;

    let TempState {
        unbonding_duration,
        receiver,
        pair_addr,
    } = TEMP_STATE.load(deps.storage)?;

    // now perform the incentive position creation
    let config = CONFIG.load(deps.storage)?;

    let pair_info: white_whale_std::pool_network::asset::PairInfo = deps.querier.query_wasm_smart(
        pair_addr.clone(),
        &white_whale_std::pool_network::pair::QueryMsg::Pair {},
    )?;

    let incentive_address: white_whale_std::pool_network::incentive_factory::IncentiveResponse =
        deps.querier.query_wasm_smart(
            config.incentive_factory_addr,
            &white_whale_std::pool_network::incentive_factory::QueryMsg::Incentive {
                lp_asset: pair_info.liquidity_token.clone(),
            },
        )?;

    // return an error if there was no incentive address
    let incentive_address = incentive_address.map_or_else(
        || {
            Err(ContractError::MissingIncentive {
                pair_address: pair_addr.to_string(),
            })
        },
        Ok,
    )?;

    // compute current LP token amount
    let mut messages = vec![];
    let mut funds = vec![];
    let lp_amount = match pair_info.liquidity_token.clone() {
        AssetInfo::NativeToken { denom } => {
            // ask the bank module
            let balance = deps.querier.query_balance(env.contract.address, denom)?;

            // deduct tax
            let asset = Asset {
                info: pair_info.liquidity_token,
                amount: balance.amount,
            };
            let balance = asset.deduct_tax(&deps.querier)?;

            // add the funds to the message
            funds.push(balance.clone());

            balance.amount
        }
        AssetInfo::Token { contract_addr } => {
            let balance: cw20::BalanceResponse = deps.querier.query_wasm_smart(
                contract_addr.clone(),
                &cw20::Cw20QueryMsg::Balance {
                    address: env.contract.address.into_string(),
                },
            )?;

            // add a message to increase allowance on the incentive contract
            // to spend our new LP tokens
            messages.push(WasmMsg::Execute {
                contract_addr,
                msg: to_json_binary(&cw20::Cw20ExecuteMsg::IncreaseAllowance {
                    spender: incentive_address.to_string(),
                    amount: balance.balance,
                    expires: None,
                })?,
                funds: vec![],
            });

            balance.balance
        }
    };

    // find out if the user has an open position with that unbonding_duration
    let positions: white_whale_std::pool_network::incentive::PositionsResponse =
        deps.querier.query_wasm_smart(
            incentive_address.clone(),
            &white_whale_std::pool_network::incentive::QueryMsg::Positions {
                address: receiver.clone().into_string(),
            },
        )?;
    let has_existing_position = positions.positions.into_iter().any(|position| {
        let QueryPosition::OpenPosition { unbonding_duration: position_unbonding_duration, .. } = position else {
            return false;
        };

        unbonding_duration == position_unbonding_duration
    });

    Ok(Response::default()
        .add_attributes(vec![
            ("action", "deposit_pair_reply"),
            ("pair_address", pair_addr.to_string().as_str()),
            ("lp_amount", &lp_amount.to_string()),
            ("unbonding_duration", &unbonding_duration.to_string()),
            ("receiver", receiver.as_ref()),
        ])
        .add_messages(messages)
        .add_message(WasmMsg::Execute {
            contract_addr: incentive_address.into_string(),
            msg: to_json_binary(&match has_existing_position {
                true => white_whale_std::pool_network::incentive::ExecuteMsg::ExpandPosition {
                    amount: lp_amount,
                    unbonding_duration,
                    receiver: Some(receiver.into_string()),
                },
                false => white_whale_std::pool_network::incentive::ExecuteMsg::OpenPosition {
                    amount: lp_amount,
                    unbonding_duration,
                    receiver: Some(receiver.into_string()),
                },
            })?,
            funds,
        }))
}
