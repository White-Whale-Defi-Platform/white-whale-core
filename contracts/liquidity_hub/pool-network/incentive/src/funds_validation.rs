use cosmwasm_std::{to_binary, Addr, Deps, Env, MessageInfo, Uint128, WasmMsg};

use crate::error::ContractError;

/// Validates that the message sender has set the specified `amount` as an
/// allowance for us to transfer for `lp_token`.
///
/// Returns the [`WasmMsg`] that will transfer the specified `amount` of the
/// `lp_token` to the contract.
pub fn validate_funds_sent(
    deps: &Deps,
    env: Env,
    lp_token: Addr,
    info: MessageInfo,
    amount: Uint128,
) -> Result<WasmMsg, ContractError> {
    let allowance: cw20::AllowanceResponse = deps.querier.query_wasm_smart(
        lp_token.clone(),
        &cw20::Cw20QueryMsg::Allowance {
            owner: info.sender.clone().into_string(),
            spender: env.contract.address.clone().into_string(),
        },
    )?;

    if allowance.allowance < amount {
        return Err(ContractError::MissingPositionDeposit {
            allowance_amount: allowance.allowance,
            deposited_amount: amount,
        });
    }

    // send the lp deposit to us
    let send_lp_deposit_msg = WasmMsg::Execute {
        contract_addr: lp_token.into_string(),
        msg: to_binary(&cw20::Cw20ExecuteMsg::TransferFrom {
            owner: info.sender.into_string(),
            recipient: env.contract.address.into_string(),
            amount,
        })?,
        funds: vec![],
    };

    Ok(send_lp_deposit_msg)
}
