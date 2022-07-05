use cosmwasm_std::{
    to_binary, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128,
    WasmMsg,
};
use cw20::{AllowanceResponse, Cw20ExecuteMsg};
use terraswap::asset::AssetInfo;

use crate::state::{BALANCES, CONFIG};

pub fn deposit(deps: DepsMut, env: Env, info: MessageInfo, amount: Uint128) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    // check that deposits are enabled
    if !config.deposit_enabled {
        return Err(StdError::generic_err("Deposits are not enabled"));
    }

    // check that user sent assets they said they did
    let sent_funds = match config.asset_info.clone() {
        AssetInfo::NativeToken { denom } => info
            .funds
            .iter()
            .filter(|c| c.denom == denom)
            .map(|c| c.amount)
            .sum::<Uint128>(),
        AssetInfo::Token { contract_addr } => {
            let allowance: AllowanceResponse = deps.querier.query_wasm_smart(
                contract_addr,
                &cw20::Cw20QueryMsg::Allowance {
                    owner: info.sender.clone().into_string(),
                    spender: env.contract.address.clone().into_string(),
                },
            )?;

            allowance.allowance
        }
    };
    if sent_funds != amount {
        return Err(StdError::generic_err(format!(
            "mismatch of sent {} but specified deposit amount of {}",
            sent_funds, amount
        )));
    }

    let mut messages: Vec<CosmosMsg> = vec![];
    // add cw20 transfer message if needed
    if let AssetInfo::Token { contract_addr } = config.asset_info {
        messages.push(
            WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.clone().into_string(),
                    recipient: env.contract.address.into_string(),
                    amount,
                })?,
                funds: vec![],
            }
            .into(),
        )
    }

    // increment user balance
    BALANCES.update::<_, StdError>(deps.storage, info.sender, |balance| {
        Ok(balance.unwrap_or_default().checked_add(amount)?)
    })?;

    Ok(Response::new()
        .add_messages(messages)
        .add_attributes(vec![("method", "deposit"), ("amount", &amount.to_string())]))
}
