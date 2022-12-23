use cosmwasm_std::{Addr, DepsMut, Order, StdError};
use cw_storage_plus::Map;

use crate::state::{TMP_VAULT_ASSET, VAULTS};
use terraswap::asset::AssetInfo;

/// Migrates the VAULTS state from v1.0.9 and lower to v1.1.0, which includes the asset info.
pub fn migrate_to_v110(deps: DepsMut) -> Result<(), StdError> {
    const VAULTSV109: Map<&[u8], Addr> = Map::new("vaults");

    // There are less than 10 vaults in each network at this point
    let vaults_v109 = VAULTSV109
        .range(deps.storage, None, None, Order::Ascending)
        .take(10)
        .collect::<Result<Vec<_>, _>>()?;

    let _ = vaults_v109.iter().map(|item| {
        let (key, vault_addr) = item;

        // All the vaults created so far are using native tokens
        // try to parse reference as a denom value
        let asset_info = AssetInfo::NativeToken {
            denom: String::from_utf8(key.to_vec())?,
        };

        VAULTS.save(
            deps.storage,
            &key.clone(),
            &(vault_addr.clone(), asset_info),
        )?;
        Ok::<(), StdError>(())
    });

    TMP_VAULT_ASSET.remove(deps.storage);

    Ok(())
}
