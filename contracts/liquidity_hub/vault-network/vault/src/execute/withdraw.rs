use cosmwasm_std::{
    coins, to_binary, BankMsg, CosmosMsg, DepsMut, MessageInfo, Response, StdError, StdResult,
    Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use terraswap::asset::AssetInfo;

use crate::state::{BALANCES, CONFIG};

pub fn withdraw(deps: DepsMut, info: MessageInfo, amount: Uint128) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    // check that withdrawals are enabled
    if !config.withdraw_enabled {
        return Err(StdError::generic_err("Withdrawals are not enabled"));
    }

    // remove from their balance
    BALANCES.update::<_, StdError>(deps.storage, info.sender.clone(), |balance| {
        Ok(balance.unwrap_or_default().checked_sub(amount)?)
    })?;

    // create message to send back to user if cw20
    let messages: Vec<CosmosMsg> = vec![match config.asset_info {
        AssetInfo::NativeToken { denom } => BankMsg::Send {
            to_address: info.sender.into_string(),
            amount: coins(amount.u128(), denom),
        }
        .into(),
        AssetInfo::Token { contract_addr } => WasmMsg::Execute {
            contract_addr,
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: info.sender.into_string(),
                amount,
            })?,
            funds: vec![],
        }
        .into(),
    }];

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("method", "withdraw"),
        ("amount", &amount.to_string()),
    ]))
}
