use cosmwasm_std::{DepsMut, Env, Response, StdError, StdResult, Uint128};
use cw20::{BalanceResponse, Cw20QueryMsg};
use terraswap::asset::AssetInfo;

use crate::state::CONFIG;

pub fn after_trade(deps: DepsMut, env: Env, old_balance: Uint128) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    // query balance
    let new_balance = match config.asset_info {
        AssetInfo::NativeToken { denom } => {
            deps.querier
                .query_balance(env.contract.address.into_string(), denom)?
                .amount
        }
        AssetInfo::Token { contract_addr } => {
            let res: BalanceResponse = deps.querier.query_wasm_smart(
                contract_addr,
                &Cw20QueryMsg::Balance {
                    address: env.contract.address.into_string(),
                },
            )?;
            res.balance
        }
    };

    // check for profit
    if new_balance < old_balance {
        return Err(StdError::generic_err(format!(
            "Final amount of {new_balance} is less than initial balance of {old_balance}",
            new_balance = new_balance,
            old_balance = old_balance
        )));
    }

    let profit = new_balance.checked_sub(old_balance)?;

    Ok(Response::new().add_attributes(vec![
        ("method", "after_trade".to_string()),
        ("profit", profit.to_string()),
    ]))
}
