use std::collections::{HashMap, HashSet};

use crate::ContractError;
use cosmwasm_std::{Addr, Deps, Uint128};
use white_whale::pool_network::asset::AssetInfo;
use white_whale::pool_network::querier::query_all_balances;
use white_whale::traits::AssetReference;
use white_whale::vault_manager::Vault;

/// Queries the balances of all assets in the vaults.
pub(crate) fn query_balances(
    deps: Deps,
    contract_address: Addr,
    vaults: &[Vault],
) -> Result<HashMap<Vec<u8>, Uint128>, ContractError> {
    let mut balances = HashMap::new();

    // get balances of all native assets in the contract, returns all non-zero balances
    let coins = query_all_balances(&deps.querier, contract_address.clone())?;
    for coin in coins {
        let asset_info = AssetInfo::NativeToken {
            denom: coin.denom.clone(),
        };
        balances.insert(asset_info.get_reference().to_vec(), coin.amount);
    }

    // Create a HashSet to track encountered AssetInfo variants
    let mut encountered_assets: HashSet<Vec<u8>> = HashSet::new();

    let filtered_vaults: Vec<Vault> = vaults
        .iter()
        .filter(|vault| {
            let is_duplicate =
                !encountered_assets.insert(vault.asset.info.clone().get_reference().to_vec());
            !is_duplicate && !balances.contains_key(vault.asset.info.get_reference())
        })
        .cloned()
        .collect();

    // this should only query balances for unique cw20 tokens as native tokens are already accounted for
    for vault in filtered_vaults {
        let balance =
            vault
                .asset
                .info
                .query_balance(&deps.querier, deps.api, contract_address.clone())?;

        balances.insert(vault.asset.info.get_reference().to_vec(), balance);
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
