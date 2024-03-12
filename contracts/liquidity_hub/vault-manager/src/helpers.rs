use std::collections::HashMap;

use cosmwasm_std::{Addr, Deps, Uint128};

use white_whale_std::pool_network::querier::query_all_balances;

use crate::ContractError;

/// Queries the balances of all assets in the vaults.
pub(crate) fn query_balances(
    deps: Deps,
    contract_address: Addr,
) -> Result<HashMap<String, Uint128>, ContractError> {
    let mut balances = HashMap::new();

    // get balances of all native assets in the contract, returns all non-zero balances
    query_all_balances(&deps.querier, contract_address)?
        .iter()
        .for_each(|coin| {
            balances.insert(coin.denom.clone(), coin.amount);
        });

    Ok(balances)
}
