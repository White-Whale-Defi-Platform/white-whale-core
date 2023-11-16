use cosmwasm_std::{
    attr, instantiate2_address, to_binary, Addr, Attribute, Binary, CodeInfoResponse, CosmosMsg,
    DepsMut, Env, HexBinary, MessageInfo, Response, StdError, WasmMsg, Uint128,
};
use cw20::MinterResponse;
use sha2::{Digest, Sha256};
use white_whale::pool_network::{
    asset::{Asset, AssetInfo, AssetInfoRaw, PairType},
    pair::PoolFee,
};

use crate::{
    helpers::{self, fill_rewards_msg},
    state::{
        add_allow_native_token, get_pair_by_identifier, ALL_TIME_BURNED_FEES,
        COLLECTABLE_PROTOCOL_FEES, PAIR_COUNTER, TOTAL_COLLECTED_PROTOCOL_FEES,
    },
    token::InstantiateMsg as TokenInstantiateMsg,
};
use crate::{
    state::{
        pair_key, Config, NAssets, NDecimals, NPairInfo as PairInfo, TmpPairInfo, MANAGER_CONFIG,
        PAIRS, TMP_PAIR_INFO,
    },
    ContractError,
};
#[cfg(any(feature = "token_factory", feature = "osmosis_token_factory"))]
use cosmwasm_std::coins;
#[cfg(any(feature = "token_factory", feature = "osmosis_token_factory"))]
use white_whale::pool_network::asset::is_factory_token;
#[cfg(feature = "token_factory")]
use white_whale::pool_network::denom::MsgCreateDenom;
#[cfg(feature = "osmosis_token_factory")]
use white_whale::pool_network::denom_osmosis::MsgCreateDenom;
use white_whale::pool_network::querier::query_balance;

#[cfg(feature = "token_factory")]
use white_whale::pool_network::denom::{Coin, MsgBurn, MsgMint};
#[cfg(feature = "osmosis_token_factory")]
use white_whale::pool_network::denom_osmosis::{Coin, MsgBurn, MsgMint};
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
    asset_infos: NAssets, //Review just a vec<asset>
    pool_fees: PoolFee,
    pair_type: PairType,
    token_factory_lp: bool,
    pair_identifier: Option<String>,
) -> Result<Response, ContractError> {
    let config: Config = MANAGER_CONFIG.load(deps.storage)?;

    // Check if fee was provided and is sufficient
    let denom = match config.pool_creation_fee.info.clone() {
        // this will never happen as the fee is always native, enforced when instantiating the contract
        AssetInfo::Token { .. } => "".to_string(),
        AssetInfo::NativeToken { denom } => denom,
    };
    if !config.pool_creation_fee.amount.is_zero() {
        // verify fee payment
        let amount = cw_utils::must_pay(&info, denom.as_str())?;
        if amount < config.pool_creation_fee.amount {
            return Err(ContractError::InvalidPairCreationFee {
                amount,
                expected: config.pool_creation_fee.amount,
            });
        }
    }

    // Prepare the sending of pair creation fee
    let mut messages: Vec<CosmosMsg> = vec![];

    // send vault creation fee to whale lair
    let creation_fee = vec![Asset {
        info: config.pool_creation_fee.info,
        amount: config.pool_creation_fee.amount,
    }];

    //send protocol fee to whale lair i.e the new fee_collector
    messages.push(fill_rewards_msg(
        config.fee_collector_addr.into_string(),
        creation_fee,
    )?);

    // Handle the asset infos and get the decimals for each asset
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
        // If we remove the TWO, THREE, N separators then the below will work for all cases
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
    // Check if the asset infos are the same
    if asset_infos_vec
        .iter()
        .any(|asset| asset_infos_vec.iter().filter(|&a| a == asset).count() > 1)
    {
        return Err(ContractError::SameAsset {});
    }

    // Verify pool fees
    pool_fees.is_valid()?;

    let pair_id = PAIR_COUNTER.load(deps.storage)?;
    // if no identifier is provided, use the vault counter (id) as identifier
    let identifier = pair_identifier.unwrap_or(pair_id.to_string());

    // check if there is an existing vault with the given identifier
    let pair = get_pair_by_identifier(&deps.as_ref(), identifier.clone());
    if pair.is_ok() {
        return Err(ContractError::PairExists {
            asset_infos: asset_infos_vec
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            identifier,
        });
    }

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
    let _lp_token_name = format!("{}-LP", asset_label);
    // TODO: Add this
    let mut messages: Vec<CosmosMsg> = vec![];
    let mut attributes = Vec::<Attribute>::new();

    let pair_creation_msg = if token_factory_lp == true {
        #[cfg(all(
            not(feature = "token_factory"),
            not(feature = "osmosis_token_factory"),
            not(feature = "injective")
        ))]
        return Err(ContractError::TokenFactoryNotEnabled {});

        let lp_symbol = format!("{asset_label}.vault.{identifier}.{LP_SYMBOL}");
        let denom = format!("{}/{}/{}", "factory", env.contract.address, lp_symbol);
        let lp_asset = AssetInfo::NativeToken { denom };

        PAIRS.save(
            deps.storage,
            identifier.clone(),
            &PairInfo {
                asset_infos: NAssets::N(asset_infos_vec),
                pair_type: pair_type.clone(),
                liquidity_token: lp_asset.clone(),
                asset_decimals: NDecimals::N(asset_decimals_vec),
                pool_fees: pool_fees,
                balances: vec![Uint128::zero(); asset_infos_vec.len()]
            },
        )?;

        attributes.push(attr("lp_asset", lp_asset.to_string()));

        #[cfg(any(
            feature = "token_factory",
            feature = "osmosis_token_factory",
            feature = "injective"
        ))]
        Ok(tokenfactory::create_denom::create_denom(
            env.contract.address,
            lp_symbol,
        ))
    } else {
        // Create the LP token using instantiate2
        let creator = deps.api.addr_canonicalize(env.contract.address.as_str())?;
        let code_id = config.token_code_id;
        let CodeInfoResponse { checksum, .. } = deps.querier.query_wasm_code_info(code_id)?;
        let seed = format!(
            "{}{}{}{}",
            asset_label,
            identifier,
            info.sender.into_string(),
            env.block.height
        );
        let salt = Binary::from(seed.as_bytes());
        let mut hasher = Sha256::new();
        hasher.update(seed.as_bytes());
        let salt = hasher.finalize().to_vec();

        // Generate the LP address with instantiate2
        let pair_lp_address = deps
            .api
            .addr_humanize(&instantiate2_address(&checksum, &creator, &salt)?)?;

        let lp_asset = AssetInfo::Token {
            contract_addr: pair_lp_address.into_string(),
        };
        // Now, after generating an address using instantiate 2 we can save this into PAIRS
        // We still need to call instantiate2 otherwise this asset will not exist, if it fails the saving will be reverted
        println!("Before save {}", lp_asset.clone());
        PAIRS.save(
            deps.storage,
            identifier.clone(),
            &PairInfo {
                asset_infos: NAssets::N(asset_infos_vec),
                pair_type: pair_type.clone(),
                liquidity_token: lp_asset.clone(),
                asset_decimals: NDecimals::N(asset_decimals_vec),
                pool_fees: pool_fees,
                balances: vec![Uint128::zero(); asset_infos_vec.len()]
            },
        )?;

        attributes.push(attr("lp_asset", lp_asset.to_string()));

        let lp_token_name = format!("{asset_label}-LP");

        Ok::<CosmosMsg, ContractError>(CosmosMsg::Wasm(WasmMsg::Instantiate2 {
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
            salt: salt.into(),
        }))
    }?;
    
    messages.push(pair_creation_msg);

    // increase pair counter
    PAIR_COUNTER.update(deps.storage, |mut counter| -> Result<_, ContractError> {
        counter += 1;
        Ok(counter)
    })?;

    attributes.push(attr("action", "create_pair"));
    attributes.push(attr("pair", &pair_label));
    attributes.push(attr("pair_label", pair_label.as_str()));
    attributes.push(attr("pair_type", pair_type.get_label()));

    // TODO: We need to store the lp addr before exiting
    Ok(Response::new()
        .add_attributes(attributes)
        .add_messages(messages))
}

/// Adds native/ibc token with decimals to the factory's whitelist so it can create pairs with that asset
pub fn add_native_token_decimals(
    deps: DepsMut,
    env: Env,
    denom: String,
    decimals: u8,
) -> Result<Response, ContractError> {
    let balance = query_balance(&deps.querier, env.contract.address, denom.to_string())?;
    if balance.is_zero() {
        return Err(ContractError::InvalidVerificationBalance {});
    }

    add_allow_native_token(deps.storage, denom.to_string(), decimals)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "add_allow_native_token"),
        ("denom", &denom),
        ("decimals", &decimals.to_string()),
    ]))
}
