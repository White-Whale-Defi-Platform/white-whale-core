use cosmwasm_std::Deps;

use white_whale::pool_network::asset::AssetInfo;
use white_whale::traits::AssetReference;
use white_whale::vault_manager::{ManagerConfig, VaultsResponse};

use crate::state::{read_vaults, MANAGER_CONFIG, VAULTS};
use crate::ContractError;

/// Gets the [ManagerConfig].
pub(crate) fn query_manager_config(deps: Deps) -> Result<ManagerConfig, ContractError> {
    Ok(MANAGER_CONFIG.load(deps.storage)?)
}

/// Gets a vault given the [AssetInfo].
pub(crate) fn query_vault(
    deps: Deps,
    asset_info: AssetInfo,
) -> Result<VaultsResponse, ContractError> {
    let vault = VAULTS
        .may_load(deps.storage, asset_info.get_reference())?
        .map_or_else(|| Err(ContractError::NonExistentVault {}), Ok)?;

    Ok(VaultsResponse {
        vaults: vec![vault],
    })
}

/// Gets all vaults in the contract.
pub(crate) fn query_vaults(
    deps: Deps,
    start_after: Option<Vec<u8>>,
    limit: Option<u32>,
) -> Result<VaultsResponse, ContractError> {
    let vaults = read_vaults(deps.storage, start_after, limit)?;

    Ok(VaultsResponse { vaults })
}
