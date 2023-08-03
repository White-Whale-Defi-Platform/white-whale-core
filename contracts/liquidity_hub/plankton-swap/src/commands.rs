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

    // TODO: may no longer be needed as we dont use the reply pattern for pair info anymore
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

// After writing create_pair I see this can get quite verbose so attempting to
// break it down into smaller modules which house some things like swap, liquidity etc
mod swap {
    // Stuff like Swap, Swap through router and any other stuff related to swapping
}

mod liquidity {
    use cosmwasm_std::{Decimal, Uint128};
    use white_whale::pool_network::asset::Asset;

    // ProvideLiquidity works based on two patterns so far and eventually 3.
    // Constant Product which is used for 2 assets
    // StableSwap which is used for 3 assets
    // Eventually concentrated liquidity will be offered but this can be assume to all be done in a well documented module we call into
    use super::*;

    pub fn provide_liquidity(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        assets: Vec<Asset>,
        slippage_tolerance: Option<Decimal>,
        receiver: Option<String>,
    ) -> Result<Response, ContractError> {
        let config = MANAGER_CONFIG.load(deps.storage)?;
        // check if the deposit feature is enabled
        if !config.feature_toggle.deposits_enabled {
            return Err(ContractError::OperationDisabled(
                "provide_liquidity".to_string(),
            ));
        }

        let (assets_vec, pools, deposits, pair_info) = match assets {
            // For TWO assets we use the constant product logic
            NAssets::TWO(assets) => {
                let pair_key = get_pair_key_from_assets(&assets, &deps)?;
                let pair_info = PAIRS.load(deps.storage, &pair_key)?;

                let mut pools: [Asset; 2] = [
                    Asset {
                        info: assets[0].clone(),
                        amount: assets[0].query_pool(
                            &deps.querier,
                            deps.api,
                            env.contract.address,
                        )?,
                    },
                    Asset {
                        info: assets[1].clone(),
                        amount: assets[1].query_pool(
                            &deps.querier,
                            deps.api,
                            env.contract.address,
                        )?,
                    },
                ];
                let deposits: [Uint128; 2] = [
                    assets
                        .iter()
                        .find(|a| a.info.equal(&pools[0].info))
                        .map(|a| a.amount)
                        .expect("Wrong asset info is given"),
                    assets
                        .iter()
                        .find(|a| a.info.equal(&pools[1].info))
                        .map(|a| a.amount)
                        .expect("Wrong asset info is given"),
                ];

                (
                    assets.to_vec(),
                    pools.to_vec(),
                    deposits.to_vec(),
                    pair_info,
                )
            }
            // For both THREE and N we use the same logic; stableswap or eventually conc liquidity
            NAssets::THREE(assets) | NAssets::N(assets) => {
                let pair_key = get_pair_key_from_assets(&assets, &deps)?;
                let pair_info = PAIRS.load(deps.storage, &pair_key)?;

                let mut pools: [Asset; 3] = [
                    Asset {
                        info: assets[0].clone(),
                        amount: assets[0].query_pool(
                            &deps.querier,
                            deps.api,
                            env.contract.address,
                        )?,
                    },
                    Asset {
                        info: assets[1].clone(),
                        amount: assets[1].query_pool(
                            &deps.querier,
                            deps.api,
                            env.contract.address,
                        )?,
                    },
                    Asset {
                        info: assets[2].clone(),
                        amount: assets[2].query_pool(
                            &deps.querier,
                            deps.api,
                            env.contract.address,
                        )?,
                    },
                ];
                let deposits: [Uint128; 3] = [
                    assets
                        .iter()
                        .find(|a| a.info.equal(&pools[0].info))
                        .map(|a| a.amount)
                        .expect("Wrong asset info is given"),
                    assets
                        .iter()
                        .find(|a| a.info.equal(&pools[1].info))
                        .map(|a| a.amount)
                        .expect("Wrong asset info is given"),
                ];

                (
                    assets.to_vec(),
                    pools.to_vec(),
                    deposits.to_vec(),
                    pair_info,
                )
            }
        };

        for asset in assets_vec.iter() {
            asset.assert_sent_native_token_balance(&info)?;
        }

        if deposits.iter().any(|&deposit| deposit.is_zero()) {
            return Err(ContractError::InvalidZeroAmount {});
        }

        let mut messages: Vec<CosmosMsg> = vec![];
        for (i, pool) in pools.iter_mut().enumerate() {
            // If the pool is token contract, then we need to execute TransferFrom msg to receive funds
            if let AssetInfo::Token { contract_addr, .. } = &pool.info {
                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract_addr.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                        owner: info.sender.to_string(),
                        recipient: env.contract.address.to_string(),
                        amount: deposits[i],
                    })?,
                    funds: vec![],
                }));
            } else {
                // If the asset is native token, balance is already increased
                // To calculate it properly we should subtract user deposit from the pool
                pool.amount = pool.amount.checked_sub(deposits[i])?;
            }
        }

        // deduct protocol fee from pools
        let collected_protocol_fees = COLLECTED_PROTOCOL_FEES.load(deps.storage)?;
        for pool in pools.iter_mut() {
            let protocol_fee =
                get_protocol_fee_for_asset(collected_protocol_fees.clone(), pool.clone().get_id());
            pool.amount = pool.amount.checked_sub(protocol_fee)?;
        }

        let liquidity_token = match pair_info.liquidity_token {
            AssetInfoRaw::Token { contract_addr } => {
                deps.api.addr_humanize(&contract_addr)?.to_string()
            }
            AssetInfoRaw::NativeToken { denom } => denom,
        };

        // Compute share and other logic based on the number of assets
        // ...

        // mint LP token to sender
        let receiver = receiver.unwrap_or_else(|| info.sender.to_string());
        messages.append(&mut mint_lp_token_msg(
            liquidity_token,
            receiver.clone(),
            env.contract.address.to_string(),
            share,
        )?);

        Ok(Response::new().add_messages(messages).add_attributes(vec![
            ("action", "provide_liquidity"),
            ("sender", info.sender.as_str()),
            ("receiver", receiver.as_str()),
            (
                "assets",
                &assets_vec
                    .iter()
                    .map(|asset| asset.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            ("share", &share.to_string()),
        ]))
    }

    fn get_pair_key_from_assets(
        assets: &[AssetInfo],
        deps: &DepsMut<'_>,
    ) -> Result<Vec<u8>, ContractError> {
        let raw_infos: Vec<AssetInfoRaw> = assets
            .iter()
            .map(|asset| asset.to_raw(deps.api))
            .collect::<Result<_, _>>()?;
        let pair_key = pair_key(&raw_infos);
        let pair_info: PairInfo = PAIRS.load(deps.storage, &pair_key)?;
        Ok(pair_key)
    }

    fn get_pools_and_deposits(
        assets: &[AssetInfo],
        deps: &DepsMut,
        env: &Env,
    ) -> Result<(Vec<Asset>, Vec<Uint128>), ContractError> {
        let mut pools = Vec::new();
        let mut deposits = Vec::new();
    
        for asset in assets.iter() {
            let amount = asset.query_pool(&deps.querier, deps.api, env.contract.address)?;
            pools.push(Asset {
                info: asset.clone(),
                amount,
            });
            deposits.push(
                assets
                    .iter()
                    .find(|a| a.info.equal(&pools.last().unwrap().info))
                    .map(|a| a.amount)
                    .expect("Wrong asset info is given"),
            );
        }
    
        Ok((pools, deposits))
    }

}

mod ownership {
    // Stuff like ProposeNewOwner, TransferOwnership, AcceptOwnership
}
