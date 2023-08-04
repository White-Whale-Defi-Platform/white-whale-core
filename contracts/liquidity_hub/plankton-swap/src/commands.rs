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
            pool_fees: pool_fees.clone(),
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
pub mod swap {
    use cosmwasm_std::Decimal;
    use white_whale::pool_network::asset::Asset;

    use crate::state::get_decimals;

    // Stuff like Swap, Swap through router and any other stuff related to swapping
    use super::*;

    pub fn swap(
        deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sender: Addr,
    offer_asset: Asset,
    ask_asset: Asset,
    belief_price: Option<Decimal>,
    max_spread: Option<Decimal>,
    to: Option<Addr>,
    ) -> Result<Response, ContractError> {
        let config = MANAGER_CONFIG.load(deps.storage)?;
        // check if the deposit feature is enabled
        if !config.feature_toggle.deposits_enabled {
            return Err(ContractError::OperationDisabled(
                "swap".to_string(),
            ));
        }

        offer_asset.assert_sent_native_token_balance(&info)?;
        
        let asset_infos = [offer_asset.info.clone(), ask_asset.info.clone()];
        let assets = [offer_asset.clone(), ask_asset.clone()];
        // Load assets, pools and pair info
        let (assets_vec, mut pools, pair_info) = match assets {
            // For TWO assets we use the constant product logic
            assets if assets.len() == 2 => {
                let pair_key = get_pair_key_from_assets(&asset_infos, &deps)?;
                let pair_info = PAIRS.load(deps.storage, &pair_key)?;

                let mut pools: [Asset; 2] = [
                    Asset {
                        info: asset_infos[0].clone(),
                        amount: asset_infos[0].query_pool(
                            &deps.querier,
                            deps.api,
                            env.contract.address.clone(),
                        )?,
                    },
                    Asset {
                        info: asset_infos[1].clone(),
                        amount: asset_infos[1].query_pool(
                            &deps.querier,
                            deps.api,
                            env.contract.address.clone(),
                        )?,
                    },
                ];

                (
                    assets.to_vec(),
                    pools.to_vec(),
                    pair_info,
                )
            }
            // For both THREE and N we use the same logic; stableswap or eventually conc liquidity
            assets if assets.len() == 3 => {
                let pair_key = get_pair_key_from_assets(&asset_infos, &deps)?;
                let pair_info = PAIRS.load(deps.storage, &pair_key)?;

                let mut pools: [Asset; 3] = [
                    Asset {
                        info: asset_infos[0].clone(),
                        amount: asset_infos[0].query_pool(
                            &deps.querier,
                            deps.api,
                            env.contract.address.clone(),
                        )?,
                    },
                    Asset {
                        info: asset_infos[1].clone(),
                        amount: asset_infos[1].query_pool(
                            &deps.querier,
                            deps.api,
                            env.contract.address.clone(),
                        )?,
                    },
                    Asset {
                        info: asset_infos[2].clone(),
                        amount: asset_infos[2].query_pool(
                            &deps.querier,
                            deps.api,
                            env.contract.address.clone(),
                        )?,
                    },
                ];

                (
                    assets.to_vec(),
                    pools.to_vec(),
                    pair_info,
                )
            }
            _ => {
                return Err(ContractError::TooManyAssets {
                    assets_provided: assets.len(),
                })
            }
        };
        // determine what's the offer and ask pool based on the offer_asset
    let offer_pool: Asset;
    let ask_pool: Asset;
    let offer_decimal: u8;
    let ask_decimal: u8;
    let decimals = get_decimals(&pair_info);

        // We now have the pools and pair info; we can now calculate the swap
        // Verify the pool 
        if offer_asset.info.equal(&pools[0].info) {
            offer_pool = pools[0].clone();
            ask_pool = pools[1].clone();
            offer_decimal = decimals[0];
            ask_decimal = decimals[1];
        } else if offer_asset.info.equal(&pools[1].info) {
            offer_pool = pools[1].clone();
            ask_pool = pools[0].clone();
    
            offer_decimal = decimals[1];
            ask_decimal = decimals[0];
        } else {
            return Err(ContractError::AssetMismatch {});
        }
    

        let mut attributes = vec![
            ("action", "swap"),
            ("pair_type", pair_info.pair_type.get_label()),
        ];

        let mut messages: Vec<CosmosMsg> = vec![];

        let receiver = to.unwrap_or_else(|| sender.clone());

        // TODO: Add the swap logic here
        let offer_amount = offer_asset.amount;
        let pool_fees = pair_info.pool_fees;
    
        // let swap_computation = helpers::compute_swap(
        //     offer_pool.amount,
        //     ask_pool.amount,
        //     offer_amount,
        //     pool_fees,
        //     &pair_info.pair_type,
        //     offer_decimal,
        //     ask_decimal,
        // )?;
        
    
        let return_asset = Asset {
            info: ask_pool.info.clone(),
            amount: swap_computation.return_amount,
        };

        // Here; add the swap messages 

        // TODO: Store the fees 



        // 1. send collateral token from the contract to a user
    // 2. stores the protocol fees
    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("action", "swap"),
        ("sender", sender.as_str()),
        ("receiver", receiver.as_str()),
        ("offer_asset", &offer_asset.info.to_string()),
        ("ask_asset", &ask_pool.info.to_string()),
        ("offer_amount", &offer_amount.to_string()),
        ("return_amount", &swap_computation.return_amount.to_string()),
        ("spread_amount", &swap_computation.spread_amount.to_string()),
        (
            "swap_fee_amount",
            &swap_computation.swap_fee_amount.to_string(),
        ),
        (
            "protocol_fee_amount",
            &swap_computation.protocol_fee_amount.to_string(),
        ),
        (
            "burn_fee_amount",
            &swap_computation.burn_fee_amount.to_string(),
        ),
        ("swap_type", pair_info.pair_type.get_label()),
    ]))
    }
}

pub mod liquidity {
    use cosmwasm_std::{Decimal, Uint128};
    use cw20::Cw20ExecuteMsg;
    use white_whale::pool_network::{
        asset::{get_total_share, Asset, MINIMUM_LIQUIDITY_AMOUNT},
        U256,
    };

    use crate::state::COLLECTABLE_PROTOCOL_FEES;

    // ProvideLiquidity works based on two patterns so far and eventually 3.
    // Constant Product which is used for 2 assets
    // StableSwap which is used for 3 assets
    // Eventually concentrated liquidity will be offered but this can be assume to all be done in a well documented module we call into
    use super::*;


    fn get_pools_and_deposits(
        assets: &[Asset],
        deps: &DepsMut,
        env: &Env,
    ) -> Result<(Vec<Asset>, Vec<Uint128>), ContractError> {
        let mut pools = Vec::new();
        let mut deposits = Vec::new();

        for asset in assets.iter() {
            let amount =
                asset
                    .info
                    .query_pool(&deps.querier, deps.api, env.contract.address.clone())?;
            pools.push(Asset {
                info: asset.info.clone(),
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

    /// Gets the protocol fee amount for the given asset_id
    pub fn get_protocol_fee_for_asset(
        collected_protocol_fees: Vec<Asset>,
        asset_id: String,
    ) -> Uint128 {
        let protocol_fee_asset = collected_protocol_fees
            .iter()
            .find(|&protocol_fee_asset| protocol_fee_asset.clone().get_id() == asset_id.clone())
            .cloned();

        // get the protocol fee for the given pool_asset
        if let Some(protocol_fee_asset) = protocol_fee_asset {
            protocol_fee_asset.amount
        } else {
            Uint128::zero()
        }
    }

    /// Creates the Mint LP message
    #[allow(unused_variables)]
    fn mint_lp_token_msg(
        liquidity_token: String,
        recipient: String,
        sender: String,
        amount: Uint128,
    ) -> Result<Vec<CosmosMsg>, ContractError> {
        #[cfg(any(feature = "token_factory", feature = "osmosis_token_factory"))]
        if is_factory_token(liquidity_token.as_str()) {
            let mut messages = vec![];
            messages.push(<MsgMint as Into<CosmosMsg>>::into(MsgMint {
                sender: sender.clone(),
                amount: Some(Coin {
                    denom: liquidity_token.clone(),
                    amount: amount.to_string(),
                }),
            }));

            if sender != recipient {
                messages.push(CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
                    to_address: recipient,
                    amount: coins(amount.u128(), liquidity_token.as_str()),
                }));
            }

            Ok(messages)
        } else {
            Ok(vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: liquidity_token,
                msg: to_binary(&Cw20ExecuteMsg::Mint { recipient, amount })?,
                funds: vec![],
            })])
        }

        #[cfg(all(not(feature = "token_factory"), not(feature = "osmosis_token_factory")))]
        Ok(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: liquidity_token,
            msg: to_binary(&Cw20ExecuteMsg::Mint { recipient, amount })?,
            funds: vec![],
        })])
    }

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

        let asset_infos = assets
            .iter()
            .map(|asset| asset.info.clone())
            .collect::<Vec<_>>();

        let (assets_vec, mut pools, deposits, pair_info) = match assets {
            // For TWO assets we use the constant product logic
            assets if assets.len() == 2 => {
                let pair_key = get_pair_key_from_assets(&asset_infos, &deps)?;
                let pair_info = PAIRS.load(deps.storage, &pair_key)?;

                let mut pools: [Asset; 2] = [
                    Asset {
                        info: asset_infos[0].clone(),
                        amount: asset_infos[0].query_pool(
                            &deps.querier,
                            deps.api,
                            env.contract.address.clone(),
                        )?,
                    },
                    Asset {
                        info: asset_infos[1].clone(),
                        amount: asset_infos[1].query_pool(
                            &deps.querier,
                            deps.api,
                            env.contract.address.clone(),
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
            assets if assets.len() == 3 => {
                let pair_key = get_pair_key_from_assets(&asset_infos, &deps)?;
                let pair_info = PAIRS.load(deps.storage, &pair_key)?;

                let mut pools: [Asset; 3] = [
                    Asset {
                        info: asset_infos[0].clone(),
                        amount: asset_infos[0].query_pool(
                            &deps.querier,
                            deps.api,
                            env.contract.address.clone(),
                        )?,
                    },
                    Asset {
                        info: asset_infos[1].clone(),
                        amount: asset_infos[1].query_pool(
                            &deps.querier,
                            deps.api,
                            env.contract.address.clone(),
                        )?,
                    },
                    Asset {
                        info: asset_infos[2].clone(),
                        amount: asset_infos[2].query_pool(
                            &deps.querier,
                            deps.api,
                            env.contract.address.clone(),
                        )?,
                    },
                ];
                let deposits: Vec<Uint128> = vec![
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
            _ => {
                return Err(ContractError::TooManyAssets {
                    assets_provided: assets.len(),
                })
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
                        recipient: env.contract.address.clone().to_string(),
                        amount: deposits[i],
                    })?,
                    funds: vec![],
                }));
            } else {
                // If the asset is native token, balance is already increased
                // To calculate it properly we should subtract user deposit from the pool
                pool.amount = pool.amount.checked_sub(deposits[i]).unwrap();
            }
        }

        // deduct protocol fee from pools
        let collected_protocol_fees =
            COLLECTABLE_PROTOCOL_FEES.load(deps.storage, &pair_info.liquidity_token.to_string())?;
        for pool in pools.iter_mut() {
            let protocol_fee =
                get_protocol_fee_for_asset(collected_protocol_fees.clone(), pool.clone().get_id());
            pool.amount = pool.amount.checked_sub(protocol_fee).unwrap();
        }

        let liquidity_token = match pair_info.liquidity_token {
            AssetInfo::Token { contract_addr } => {
                deps.api.addr_validate(&contract_addr)?.to_string()
            }
            AssetInfo::NativeToken { denom } => denom,
        };

        // Compute share and other logic based on the number of assets
        let share = Uint128::zero();
        // ...
        let total_share = get_total_share(&deps.as_ref(), liquidity_token.clone())?;

        let share = match pair_info.pair_type {
            PairType::ConstantProduct => {
                let share = if total_share == Uint128::zero() {
                    // Make sure at least MINIMUM_LIQUIDITY_AMOUNT is deposited to mitigate the risk of the first
                    // depositor preventing small liquidity providers from joining the pool
                    let share = Uint128::new(
                        (U256::from(deposits[0].u128())
                            .checked_mul(U256::from(deposits[1].u128()))
                            .ok_or::<ContractError>(ContractError::LiquidityShareComputation {}))?
                        .integer_sqrt()
                        .as_u128(),
                    )
                    .checked_sub(MINIMUM_LIQUIDITY_AMOUNT)
                    .map_err(|_| {
                        ContractError::InvalidInitialLiquidityAmount(MINIMUM_LIQUIDITY_AMOUNT)
                    })?;

                    messages.append(&mut mint_lp_token_msg(
                        liquidity_token.clone(),
                        env.contract.address.to_string(),
                        env.contract.address.to_string(),
                        share,
                    )?);

                    share
                } else {
                    let share = {
                        let numerator = U256::from(deposits[0].u128())
                            .checked_mul(U256::from(total_share.u128()))
                            .ok_or::<ContractError>(ContractError::LiquidityShareComputation {})?;

                        let denominator = U256::from(pools[0].amount.u128());

                        let result = numerator
                            .checked_div(denominator)
                            .ok_or::<ContractError>(ContractError::LiquidityShareComputation {})?;

                        Uint128::from(result.as_u128())
                    };

                    messages.append(&mut mint_lp_token_msg(
                        liquidity_token.clone(),
                        env.contract.address.to_string(),
                        env.contract.address.to_string(),
                        share,
                    )?);

                    share
                };
                share
            }
            PairType::StableSwap { amp } => {
                let share = Uint128::one();
                // TODO: Handle stableswap

                share
            }
        };

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
}

pub mod ownership {
    // Stuff like ProposeNewOwner, TransferOwnership, AcceptOwnership
    use super::*;
}
