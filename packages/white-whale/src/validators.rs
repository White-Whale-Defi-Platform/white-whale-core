use cosmwasm_std::{BalanceResponse, BankQuery, Coin, Deps, Env, QueryRequest, StdError, StdResult, Uint128};

/// Validates that the given denom is sent in the funds.
pub fn validate_denom(deps: Deps, env: &Env, denom: &String, funds: &Vec<Coin>) -> StdResult<()> {
    let balance_response: BalanceResponse =
        deps.querier.query(&QueryRequest::Bank(BankQuery::Balance {
            address: env.contract.address.to_string(),
            denom: denom.clone(),
        }))?;

    for coin in funds {
        if coin.denom == balance_response.amount.denom && balance_response.amount.amount > Uint128::zero() {
            return Ok(());
        }
    }

    Err(StdError::generic_err(format!("Denom {} not found in funds", denom)))
}