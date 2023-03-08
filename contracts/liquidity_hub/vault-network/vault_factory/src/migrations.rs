use cosmwasm_std::{Addr, DepsMut, Order, StdError};
use cw_storage_plus::Map;

use pool_network::asset::AssetInfo;

use crate::state::{TMP_VAULT_ASSET, VAULTS};

/// Migrates the VAULTS state from v1.0.9 and lower to v1.1.0, which includes the asset info.
pub fn migrate_to_v110(deps: DepsMut) -> Result<(), StdError> {
    const VAULTSV109: Map<&[u8], Addr> = Map::new("vaults");

    // There are less than 10 vaults in each network at this point
    let vaults_v109 = VAULTSV109
        .range(deps.storage, None, None, Order::Ascending)
        .take(10)
        .collect::<Result<Vec<_>, _>>()?;

    vaults_v109
        .into_iter()
        .try_for_each(|(key, vault_addr)| -> Result<(), StdError> {
            // All the vaults created so far are using native tokens
            // try to parse reference as a denom value
            let asset_info = AssetInfo::NativeToken {
                denom: String::from_utf8(key.clone())?,
            };

            VAULTS.save(deps.storage, &key, &(vault_addr, asset_info))?;

            Ok(())
        })?;

    TMP_VAULT_ASSET.remove(deps.storage);

    Ok(())
}
