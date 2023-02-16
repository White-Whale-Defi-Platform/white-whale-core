use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, DepsMut, Order, StdResult};
use cw_storage_plus::{Item, Map};
use pool_network::asset::AssetInfo;

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub pool_router: Addr,
}

pub type ConfigResponse = Config;

pub const CONFIG: Item<Config> = Item::new("config");
pub const TMP_ASSET_INFOS: Map<String, AssetInfo> = Map::new("tmp_asset_infos");

pub fn store_temporal_asset_info(deps: DepsMut, asset_info: AssetInfo) -> StdResult<()> {
    let key = asset_info
        .clone()
        .get_label(&deps.as_ref())
        .expect("Couldn't get assetinfo label");

    TMP_ASSET_INFOS.save(deps.storage, key, &asset_info)
}

pub fn read_temporal_asset_infos(deps: &mut DepsMut) -> StdResult<Vec<AssetInfo>> {
    let mut asset_infos = vec![];
    for item in TMP_ASSET_INFOS.range(deps.storage, None, None, Order::Ascending) {
        let (_, asset_info) = item?;
        asset_infos.push(asset_info);
    }

    TMP_ASSET_INFOS.clear(deps.storage);

    Ok(asset_infos)
}
