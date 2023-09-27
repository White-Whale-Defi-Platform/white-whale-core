use cosmwasm_std::{Decimal, Deps, Env, Uint128, Uint256};

use white_whale::pool_network::asset::{get_total_share, Asset, AssetInfo};
use white_whale::traits::AssetReference;
use white_whale::vault_manager::{Config, PaybackAssetResponse, ShareResponse, VaultsResponse};

use crate::state::{get_vault, read_vaults, CONFIG, VAULTS};
use crate::ContractError;

/// Gets the [Config].
pub(crate) fn query_manager_config(deps: Deps) -> Result<Config, ContractError> {
    Ok(CONFIG.load(deps.storage)?)
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

/// Gets the share of the assets stored in the vault that a given `lp_share` is entitled to.
pub(crate) fn get_share(
    deps: Deps,
    env: Env,
    lp_asset: Asset,
) -> Result<ShareResponse, ContractError> {
    let vault = get_vault(&deps, &lp_asset.info)?;

    let lp_amount = get_total_share(&deps, lp_asset.info.to_string())?;
    let balance = vault
        .asset_info
        .query_balance(&deps.querier, deps.api, env.contract.address)?;

    // lp_share = amount / lp_amount
    // asset_share = lp_share * balance
    let asset_share = Decimal::from_ratio(lp_asset.amount, lp_amount) * balance;
    Ok(ShareResponse {
        share: Asset {
            info: vault.asset_info,
            amount: asset_share,
        },
    })
}

/// Gets payback amount for a given asset.
pub(crate) fn get_payback_amount(
    deps: Deps,
    asset: Asset,
) -> Result<PaybackAssetResponse, ContractError> {
    let vault = VAULTS
        .may_load(deps.storage, asset.info.get_reference())?
        .map_or_else(|| Err(ContractError::NonExistentVault {}), Ok)?;

    // check that balance is greater than expected
    let protocol_fee =
        Uint128::try_from(vault.fees.protocol_fee.compute(Uint256::from(asset.amount)))?;
    let flash_loan_fee = Uint128::try_from(
        vault
            .fees
            .flash_loan_fee
            .compute(Uint256::from(asset.amount)),
    )?;

    let required_amount = asset
        .amount
        .checked_add(protocol_fee)?
        .checked_add(flash_loan_fee)?;

    Ok(PaybackAssetResponse {
        asset_info: asset.info,
        payback_amount: required_amount,
        protocol_fee,
        flash_loan_fee,
    })
}
