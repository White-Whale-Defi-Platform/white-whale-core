use cosmwasm_std::{coins, to_binary, Addr, BankMsg, CosmosMsg, DepsMut, Env, Response, WasmMsg};
use terraswap::asset::{Asset, AssetInfo};
use vault_network::vault::PaybackAmountResponse;

use crate::err::{StdResult, VaultRouterError};

pub fn complete_loan(
    deps: DepsMut,
    env: Env,
    initiator: Addr,
    assets: Vec<(String, Asset)>,
) -> StdResult<Response> {
    // pay back loans and profit
    let messages: Vec<Vec<CosmosMsg>> = assets
        .into_iter()
        .map(|(vault, loaned_asset)| {
            let payback_amount: PaybackAmountResponse = deps.querier.query_wasm_smart(
                vault.clone(),
                &vault_network::vault::QueryMsg::GetPaybackAmount {
                    amount: loaned_asset.amount,
                },
            )?;

            // calculate amount router has after performing flash-loan
            let final_amount = match &loaned_asset.info {
                AssetInfo::NativeToken { denom } => {
                    let amount = deps
                        .querier
                        .query_balance(env.contract.address.clone(), denom)?;

                    amount.amount
                }
                AssetInfo::Token { contract_addr } => {
                    let res: cw20::BalanceResponse = deps.querier.query_wasm_smart(
                        contract_addr,
                        &cw20::Cw20QueryMsg::Balance {
                            address: env.contract.address.clone().into_string(),
                        },
                    )?;

                    res.balance
                }
            };

            let profit_amount = final_amount
                .checked_sub(payback_amount.payback_amount)
                .map_err(|_| VaultRouterError::NegativeProfit {
                    input: loaned_asset.clone(),
                    output_amount: final_amount,
                    required_amount: payback_amount.payback_amount,
                })?;

            let mut response_messages: Vec<CosmosMsg> = vec![];
            let payback_loan_msg: StdResult<CosmosMsg> = match loaned_asset.info.clone() {
                AssetInfo::NativeToken { denom } => Ok(BankMsg::Send {
                    to_address: vault,
                    amount: coins(payback_amount.payback_amount.u128(), denom),
                }
                .into()),
                AssetInfo::Token { contract_addr } => Ok(WasmMsg::Execute {
                    contract_addr,
                    funds: vec![],
                    msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                        recipient: vault,
                        amount: payback_amount.payback_amount,
                    })?,
                }
                .into()),
            };

            response_messages.push(payback_loan_msg?);

            // add profit message if non-zero profit
            if !profit_amount.is_zero() {
                let profit_payback_msg: StdResult<CosmosMsg> = match loaned_asset.info {
                    AssetInfo::NativeToken { denom } => Ok(BankMsg::Send {
                        to_address: initiator.clone().into_string(),
                        amount: coins(profit_amount.u128(), denom),
                    }
                    .into()),
                    AssetInfo::Token { contract_addr } => Ok(WasmMsg::Execute {
                        contract_addr,
                        funds: vec![],
                        msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                            recipient: initiator.clone().into_string(),
                            amount: profit_amount,
                        })?,
                    }
                    .into()),
                };

                response_messages.push(profit_payback_msg?);
            }

            Ok(response_messages)
        })
        .collect::<StdResult<Vec<Vec<_>>>>()?;

    Ok(Response::new()
        .add_messages(messages.concat())
        .add_attributes(vec![("method", "complete_loan")]))
}

#[cfg(test)]
mod tests {}
