use classic_bindings::TerraQuery;
use cosmwasm_std::{QuerierWrapper, StdResult};

use white_whale_std::pool_network::asset::{Asset, AssetInfo};

pub fn deduct_tax_vec(
    querier: &QuerierWrapper<TerraQuery>,
    assets: &[Asset],
) -> StdResult<Vec<Asset>> {
    let mut discounted_assets = Vec::with_capacity(assets.len());

    for asset in assets {
        let mut discounted_asset = asset.clone();

        if let AssetInfo::NativeToken { .. } = &discounted_asset.info {
            discounted_asset.amount = discounted_asset
                .amount
                .checked_sub(discounted_asset.compute_tax(querier)?)?;
        }

        discounted_assets.push(discounted_asset);
    }

    Ok(discounted_assets)
}
