use cosmwasm_std::{
    attr, instantiate2_address, to_binary, Addr, Attribute, Binary, CodeInfoResponse, CosmosMsg,
    DepsMut, Env, MessageInfo, Response, StdError, WasmMsg,
};
use cw20::MinterResponse;
use white_whale::pool_network::{
    asset::{AssetInfo, AssetInfoRaw, PairType},
    pair::{FeatureToggle, PoolFee},
};

use crate::token::InstantiateMsg as TokenInstantiateMsg;
use crate::{
    state::{
        pair_key, Config, NAssets, NDecimals, NPairInfo as PairInfo, TmpPairInfo, MANAGER_CONFIG,
        PAIRS, TMP_PAIR_INFO,
    },
    ContractError,
};
pub const MAX_ASSETS_PER_POOL: usize = 4;
pub const LP_SYMBOL: &str = "uLP";

/// Creates a liquidity pool pair with 2, 3, or N assets. The function dynamically handles different numbers of assets,
/// allowing for the creation of pairs with varying configurations. The maximum number of assets per pool is defined by
/// the constant `MAX_ASSETS_PER_POOL`.
///
/// # Example
///
/// ```rust
/// # use cosmwasm_std::{DepsMut, Decimal, Env, MessageInfo, Response, CosmosMsg, WasmMsg, to_binary};
/// # use white_whale::pool_network::{asset::{AssetInfo, PairType}, pair::PoolFee};
/// # use white_whale::fee::Fee;
/// # use plankton_swap::state::{NAssets};
/// # use plankton_swap::error::ContractError;
/// # use plankton_swap::commands::MAX_ASSETS_PER_POOL;
/// # use plankton_swap::commands::create_pair;
/// # use std::convert::TryInto;
/// #
/// # fn example(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
/// let asset_infos = NAssets::TWO([
///     AssetInfo::NativeToken { denom: "uatom".into() },
///     AssetInfo::NativeToken { denom: "uscrt".into() },
/// ]);
/// let pool_fees = PoolFee {
///     protocol_fee: Fee {
///         share: Decimal::percent(5u64),
///     },
///     swap_fee: Fee {
///         share: Decimal::percent(7u64),
///     },
///     burn_fee: Fee {
///         share: Decimal::zero(),
///     },
/// };
/// let pair_type = PairType::ConstantProduct;
/// let token_factory_lp = false;
///
/// let response = create_pair(deps, env, info, asset_infos, pool_fees, pair_type, token_factory_lp)?;
/// # Ok(response)
/// # }
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
        .any(|asset| asset_infos_vec.iter().filter(|&a| a == asset).count() > 1)
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
            pair_key: pair_key.clone(),
            asset_infos: NAssets::N(asset_infos_vec.clone()),
            asset_decimals: crate::state::NDecimals::N(asset_decimals_vec.clone()),
            pair_type: pair_type.clone(),
        },
    )?;

    // prepare labels for creating the pair token with a meaningful name
    let pair_label = asset_infos_vec
        .iter()
        .map(|asset| asset.to_owned().get_label(&deps.as_ref()))
        .collect::<Result<Vec<_>, _>>()?
        .join("-");
    // Convert asset_infos_vec into the type [AssetInfo; 2] to avoid the error expected array `[AssetInfo; 2]`
    //   found struct `std::vec::Vec<AssetInfo>`
    // Now instead of sending a SubMsg to create the pair we can just call the instantiate function for an LP token
    // and save the info in PAIRS using pairkey as the key

    let asset_labels: Result<Vec<String>, _> = asset_infos_vec
        .iter()
        .map(|asset| asset.clone().get_label(&deps.as_ref()))
        .collect();

    let asset_label = asset_labels?.join("-"); // Handle the error if needed
    let lp_token_name = format!("{}-LP", asset_label);
    // TODO: Add this
    // helpers::create_lp_token(deps, &env, &msg, &lp_token_name);

    let mut attributes = Vec::<Attribute>::new();

    // Create the LP token using instantiate2
    let creator = deps.api.addr_canonicalize(env.contract.address.as_str())?;
    let code_id = config.token_code_id;
    let CodeInfoResponse { checksum, .. } = deps.querier.query_wasm_code_info(code_id)?;
    let seed = format!(
        "{}{}{}",
        asset_label,
        info.sender.into_string(),
        env.block.height
    );
    let salt = Binary::from(seed.as_bytes());

    let pool_lp_address = deps.api.addr_humanize(
        &instantiate2_address(&checksum, &creator, &salt)
            .map_err(|e| StdError::generic_err(e.to_string()))?,
    )?;

    let lp_asset = AssetInfo::Token {
        contract_addr: pool_lp_address.into_string(),
    };

    // Now, after generating an address using instantiate 2 we can save this into PAIRS
    // We still need to call instantiate2 otherwise this asset will not exist, if it fails the saving will be reverted
    PAIRS.save(
        deps.storage,
        &pair_key,
        &PairInfo {
            asset_infos: NAssets::N(asset_infos_vec.clone()),
            pair_type: pair_type.clone(),
            liquidity_token: lp_asset.clone(),
            asset_decimals: NDecimals::N(asset_decimals_vec.clone()),
        },
    )?;

    attributes.push(attr("lp_asset", lp_asset.to_string()));

    let lp_token_name = format!("{asset_label}-LP");

    let message = CosmosMsg::Wasm(WasmMsg::Instantiate2 {
        admin: None,
        code_id,
        label: lp_token_name.to_owned(),
        msg: to_binary(&TokenInstantiateMsg {
            name: lp_token_name,
            symbol: LP_SYMBOL.to_string(),
            decimals: 6,
            initial_balances: vec![],
            mint: Some(MinterResponse {
                minter: env.contract.address.to_string(),
                cap: None,
            }),
        })?,
        funds: vec![],
        salt,
    });

    // TODO: We need to store the lp addr before exiting
    Ok(Response::new()
        .add_attributes(vec![
            ("action", "create_pair"),
            ("pair", &pair_label),
            ("pair_label", pair_label.as_str()),
            ("pair_type", pair_type.get_label()),
        ])
        .add_message(message))
}
