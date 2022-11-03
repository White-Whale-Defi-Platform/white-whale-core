use cosmwasm_std::{to_binary, Binary, Deps, Uint128};
use vault_network::vault::PaybackAmountResponse;

use crate::{error::StdResult, state::CONFIG};

pub fn get_payback_amount(deps: Deps, amount: Uint128) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;

    // check that balance is greater than expected
    let protocol_fee = Uint128::from(
        config
            .fees
            .protocol_fee
            .compute(cosmwasm_bignumber::Uint256::from(amount)),
    );
    let flash_loan_fee = Uint128::from(
        config
            .fees
            .flash_loan_fee
            .compute(cosmwasm_bignumber::Uint256::from(amount)),
    );

    let required_amount = amount
        .checked_add(protocol_fee)?
        .checked_add(flash_loan_fee)?;

    Ok(to_binary(&PaybackAmountResponse {
        payback_amount: required_amount,
        protocol_fee,
        flash_loan_fee,
    })?)
}
