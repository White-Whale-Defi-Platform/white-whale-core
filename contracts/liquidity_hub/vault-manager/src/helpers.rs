use std::collections::HashMap;

use cosmwasm_std::{to_binary, Addr, CosmosMsg, Deps, Uint128, WasmMsg};

use white_whale::pool_network::asset::{Asset, AssetInfo, ToCoins};
use white_whale::traits::AssetReference;
use white_whale::vault_manager::Vault;
use white_whale::whale_lair;

use crate::ContractError;

/// Creates a message to fill rewards on the whale lair contract.
pub(crate) fn fill_rewards_msg(
    contract_addr: String,
    assets: Vec<Asset>,
) -> Result<CosmosMsg, ContractError> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr,
        msg: to_binary(&whale_lair::ExecuteMsg::FillRewards {
            assets: assets.clone(),
        })?,
        funds: assets.to_coins()?,
    }))
}

/// Queries the balances of all assets in the vaults.
pub(crate) fn query_balances<'a>(
    deps: Deps,
    contract_address: Addr,
    vaults: &'a Vec<Vault>,
) -> Result<HashMap<&'a [u8], Uint128>, ContractError> {
    let mut balances = HashMap::new();
    for vault in vaults {
        let balance =
            vault
                .asset
                .info
                .query_balance(&deps.querier, deps.api, contract_address.clone())?;

        // aggregate balances of the same asset
        if balances.contains_key(vault.asset.info.get_reference()) {
            let current_value: &Uint128 = balances
                .get(vault.asset.info.get_reference())
                .ok_or(ContractError::NonExistentVault {})?;
            balances.insert(
                vault.asset.info.get_reference(),
                balance.checked_add(current_value.to_owned())?,
            );
        } else {
            balances.insert(vault.asset.info.get_reference(), balance);
        }
    }

    Ok(balances)
}

/// Assets that the provided asset is held in the given vault.
pub(crate) fn assert_asset(expected: &AssetInfo, actual: &AssetInfo) -> Result<(), ContractError> {
    if expected != actual {
        return Err(ContractError::AssetMismatch {
            expected: expected.to_string(),
            actual: actual.to_string(),
        });
    }
    Ok(())
}
