use cosmwasm_std::{
    to_binary, CosmosMsg, DepsMut, Env, MessageInfo, ReplyOn, Response, SubMsg, WasmMsg,
};
use white_whale::pool_network::{
    asset::{AssetInfo, AssetInfoRaw, PairType},
    pair::PoolFee,
};

use crate::{
    state::{pair_key, Config, NAssets, TmpPairInfo, MANAGER_CONFIG, PAIRS, TMP_PAIR_INFO},
    ContractError,
};
pub const MAX_ASSETS_PER_POOL: usize = 4;
use white_whale::pool_network::pair::{
    FeatureToggle, InstantiateMsg as PairInstantiateMsg, MigrateMsg as PairMigrateMsg,
};
/// Creates a Pair, we want this to be dynamic such that we can use this one entrypoint to
/// create a pair with 2 assets, 3 assets or eventually N. For N we enforce 4 as max for now and we want use
///
/// ```rust
/// #[cw_serde]
// pub enum NAssets {
//     TWO([AssetInfoRaw; 2]),
//     THREE([AssetInfoRaw; 3]),
//     // N Assets is also possible where N is the number of assets in the pool
//     // Note Vec with an unbounded size, we need to have extra parsing on this one to eventually store [AssetInfoRaw; N]
//     N(Vec<AssetInfoRaw>),
// }

// #[cw_serde]
// pub enum NDecimals {
//     TWO([u8; 2]),
//     THREE([u8; 3]),
// }
///
/// ```
pub fn create_pair(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset_infos: NAssets,
    pool_fees: PoolFee,
    pair_type: PairType,
    token_factory_lp: bool,
) -> Result<Response, ContractError> {
    let config: Config = MANAGER_CONFIG.load(deps.storage)?;

    let (asset_infos_vec, asset_decimals_vec) = match asset_infos {
        NAssets::TWO(assets) => {
            let decimals = [
                assets[0].query_decimals(env.contract.address.clone(), &deps.querier)?,
                assets[1].query_decimals(env.contract.address.clone(), &deps.querier)?,
            ];
            (assets.to_vec(), decimals.to_vec())
        }
        NAssets::THREE(assets) => {
            let decimals = [
                assets[0].query_decimals(env.contract.address.clone(), &deps.querier)?,
                assets[1].query_decimals(env.contract.address.clone(), &deps.querier)?,
                assets[2].query_decimals(env.contract.address.clone(), &deps.querier)?,
            ];
            (assets.to_vec(), decimals.to_vec())
        }
        NAssets::N(assets) => {
            if assets.len() > MAX_ASSETS_PER_POOL {
                return Err(ContractError::TooManyAssets {
                    assets_provided: assets.len(),
                });
            }
            let decimals: Vec<u8> = assets
                .iter()
                .map(|asset| asset.query_decimals(env.contract.address.clone(), &deps.querier))
                .collect::<Result<_, _>>()?;
            (assets, decimals)
        }
    };

    if asset_infos_vec
        .iter()
        .any(|&asset| asset_infos_vec.iter().filter(|&&a| a == asset).count() > 1)
    {
        return Err(ContractError::SameAsset {});
    }

    let raw_infos: Vec<AssetInfoRaw> = asset_infos_vec
        .iter()
        .map(|asset| asset.to_raw(deps.api))
        .collect::<Result<_, _>>()?;

    let pair_key = pair_key(&raw_infos);
    if let Ok(Some(_)) = PAIRS.may_load(deps.storage, &pair_key) {
        return Err(ContractError::ExistingPair {});
    }

    TMP_PAIR_INFO.save(
        deps.storage,
        &TmpPairInfo {
            pair_key,
            asset_infos: NAssets::N(asset_infos_vec.clone()),
            asset_decimals: crate::state::NDecimals::N(asset_decimals_vec.clone()),
            pair_type: pair_type.clone(),
        },
    )?;

    // prepare labels for creating the pair token with a meaningful name
    let pair_label = asset_infos_vec
        .iter()
        .map(|asset| asset.get_label(&deps.as_ref()))
        .collect::<Result<Vec<_>, _>>()?
        .join("-");
    // Convert asset_infos_vec into the type [AssetInfo; 2] to avoid the error expected array `[AssetInfo; 2]`
    //   found struct `std::vec::Vec<AssetInfo>`
    // Generalize this to N too, do we need to update pair ? to handle N assets, is there a trio pair? If so then yes
    let thevec: [AssetInfo; 2] = [asset_infos_vec[0].clone(), asset_infos_vec[1].clone()];
    let thedecimals: [u8; 2] = [asset_decimals_vec[0].clone(), asset_decimals_vec[1].clone()];
    Ok(Response::new()
        .add_attributes(vec![
            ("action", "create_pair"),
            ("pair", &pair_label),
            ("pair_label", pair_label.as_str()),
            ("pair_type", pair_type.get_label()),
        ])
        .add_submessage(SubMsg {
            id: 1u64,
            gas_limit: None,
            msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
                code_id: config.pair_code_id,
                funds: info.funds,
                admin: Some(env.contract.address.to_string()),
                label: pair_label,
                msg: to_binary(&PairInstantiateMsg {
                    asset_infos: thevec,
                    token_code_id: config.token_code_id,
                    asset_decimals: thedecimals,
                    pool_fees,
                    fee_collector_addr: config.fee_collector_addr.to_string(),
                    pair_type,
                    token_factory_lp,
                })?,
            }),
            reply_on: ReplyOn::Success,
        }))
}
