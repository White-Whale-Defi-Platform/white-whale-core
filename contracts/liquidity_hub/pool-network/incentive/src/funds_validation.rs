use cosmwasm_std::{to_binary, Deps, Env, MessageInfo, Uint128, WasmMsg};
use cw_utils::PaymentError;
use white_whale::pool_network::asset::AssetInfo;

use crate::error::ContractError;

/// Validates that the message sender has sent the tokens to the contract.
/// In case the `lp_token` is a cw20 token, check if the sender set the specified `amount` as an
/// allowance for us to transfer for `lp_token`.
///
/// If `lp_token` is a native token, check if the funds were sent in the [`MessageInfo`] struct.
///
/// Returns the [`WasmMsg`] that will transfer the specified `amount` of the
/// `lp_token` to the contract.
pub fn validate_funds_sent(
    deps: &Deps,
    env: Env,
    lp_token: AssetInfo,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Option<WasmMsg>, ContractError> {
    if amount.is_zero() {
        return Err(ContractError::PaymentError(PaymentError::NoFunds {}));
    }

    let send_lp_deposit_msg = match lp_token {
        AssetInfo::Token { contract_addr } => {
            let allowance: cw20::AllowanceResponse = deps.querier.query_wasm_smart(
                contract_addr.clone(),
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
            Some(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&cw20::Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.into_string(),
                    recipient: env.contract.address.into_string(),
                    amount,
                })?,
                funds: vec![],
            })
        }
        AssetInfo::NativeToken { denom } => {
            let paid_amount = cw_utils::must_pay(&info, &denom)?;
            if paid_amount != amount {
                return Err(ContractError::MissingPositionDepositNative {
                    desired_amount: amount,
                    deposited_amount: paid_amount,
                });
            }
            // no message as native tokens are transferred in the `info` struct
            None
        }
    };

    Ok(send_lp_deposit_msg)
}
