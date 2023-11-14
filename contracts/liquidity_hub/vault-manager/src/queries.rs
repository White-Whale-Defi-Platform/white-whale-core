use cosmwasm_std::{Decimal, Deps, Uint128, Uint256};

use white_whale::pool_network::asset::{get_total_share, Asset};
use white_whale::vault_manager::{
    Config, FilterVaultBy, PaybackAssetResponse, ShareResponse, VaultsResponse,
};

use crate::state::{
    get_vault_by_identifier, get_vault_by_lp, get_vaults, get_vaults_by_asset_info, CONFIG,
};
use crate::ContractError;

/// Gets the [Config].
pub(crate) fn query_manager_config(deps: Deps) -> Result<Config, ContractError> {
    Ok(CONFIG.load(deps.storage)?)
}

/// Gets a vault given the params provided by the filter [FilterVaultBy].
pub(crate) fn query_vault(
    deps: Deps,
    filter_by: FilterVaultBy,
) -> Result<VaultsResponse, ContractError> {
    let vaults = match filter_by {
        FilterVaultBy::AssetInfo(params) => get_vaults_by_asset_info(
            deps.storage,
            params.asset_info,
            params.start_after,
            params.limit,
        )?,
        FilterVaultBy::Identifier(params) => {
            vec![get_vault_by_identifier(&deps, params.identifier)?]
        }
        FilterVaultBy::LpAsset(params) => vec![get_vault_by_lp(&deps, &params.lp_asset)?],
    };

    Ok(VaultsResponse { vaults })
}

/// Gets all vaults in the contract.
pub(crate) fn query_vaults(
    deps: Deps,
    start_after: Option<Vec<u8>>,
    limit: Option<u32>,
) -> Result<VaultsResponse, ContractError> {
    let vaults = get_vaults(deps.storage, start_after, limit)?;

    Ok(VaultsResponse { vaults })
}

/// Gets the share of the assets stored in the vault that a given `lp_share` is entitled to.
pub(crate) fn get_share(deps: Deps, lp_asset: Asset) -> Result<ShareResponse, ContractError> {
    let vault = get_vault_by_lp(&deps, &lp_asset.info)?;

    let lp_amount = get_total_share(&deps, lp_asset.info.to_string())?;

    // lp_share = amount / lp_amount
    // asset_share = lp_share * vault.asset.amount
    let asset_share = Decimal::from_ratio(lp_asset.amount, lp_amount) * vault.asset.amount;
    Ok(ShareResponse {
        share: Asset {
            info: vault.asset.info,
            amount: asset_share,
        },
    })
}

/// Gets payback amount for a given asset.
pub(crate) fn get_payback_amount(
    deps: Deps,
    asset: Asset,
    vault_identifier: String,
) -> Result<PaybackAssetResponse, ContractError> {
    let vault = get_vault_by_identifier(&deps, vault_identifier)?;

    // sanity check
    if vault.asset.amount < asset.amount {
        return Err(ContractError::InsufficientAssetBalance {
            asset_balance: vault.asset.amount,
            requested_amount: asset.amount,
        });
    }

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
