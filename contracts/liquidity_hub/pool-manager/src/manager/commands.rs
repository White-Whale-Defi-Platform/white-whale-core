use cosmwasm_std::{
    attr, instantiate2_address, to_json_binary, Attribute, CodeInfoResponse, CosmosMsg, DepsMut,
    Env, MessageInfo, Response, Uint128, WasmMsg,
};
use cw20::MinterResponse;
use sha2::{Digest, Sha256};
use white_whale_std::{
    pool_network::{
        asset::{Asset, AssetInfo, PairType},
        pair::PoolFee,
    },
    whale_lair::fill_rewards_msg,
};

use crate::{
    state::{add_allow_native_token, get_pair_by_identifier, PAIR_COUNTER},
    token::InstantiateMsg as TokenInstantiateMsg,
};
use crate::{
    state::{Config, MANAGER_CONFIG, PAIRS},
    ContractError,
};
#[cfg(any(feature = "token_factory", feature = "osmosis_token_factory"))]
use cosmwasm_std::coins;
use white_whale_std::pool_manager::NPairInfo as PairInfo;
#[cfg(any(feature = "token_factory", feature = "osmosis_token_factory"))]
use white_whale_std::pool_network::asset::is_factory_token;
#[cfg(feature = "token_factory")]
use white_whale_std::pool_network::denom::MsgCreateDenom;
#[cfg(feature = "osmosis_token_factory")]
use white_whale_std::pool_network::denom_osmosis::MsgCreateDenom;
use white_whale_std::pool_network::querier::query_balance;

#[cfg(feature = "token_factory")]
use white_whale_std::pool_network::denom::{Coin, MsgBurn, MsgMint};
#[cfg(feature = "osmosis_token_factory")]
use white_whale_std::pool_network::denom_osmosis::{Coin, MsgBurn, MsgMint};
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
/// # use white_whale_std::pool_network::{asset::{AssetInfo, PairType}, pair::PoolFee};
/// # use white_whale_std::fee::Fee;
/// # use pool_manager::error::ContractError;
/// # use pool_manager::manager::commands::MAX_ASSETS_PER_POOL;
/// # use pool_manager::manager::commands::create_pair;
/// # use std::convert::TryInto;
/// #
/// # fn example(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
/// let asset_infos = vec![
///     AssetInfo::NativeToken { denom: "uatom".into() },
///     AssetInfo::NativeToken { denom: "uscrt".into() },
/// ];
/// #[cfg(not(feature = "osmosis"))]
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
///
/// #[cfg(feature = "osmosis")]
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
///     osmosis_fee: Fee {
///         share: Decimal::zero(),
///     },
/// };
/// let pair_type = PairType::ConstantProduct;
/// let token_factory_lp = false;
///
/// let response = create_pair(deps, env, info, asset_infos, pool_fees, pair_type, token_factory_lp, None)?;
/// # Ok(response)
/// # }
/// ```
#[allow(clippy::too_many_arguments)]
pub fn create_pair(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset_infos: Vec<AssetInfo>, //Review just a vec<asset>
    pool_fees: PoolFee,
    pair_type: PairType,
    token_factory_lp: bool,
    pair_identifier: Option<String>,
) -> Result<Response, ContractError> {
    // Load config for pool creation fee
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

    // //send protocol fee to whale lair i.e the new fee_collector
    messages.push(fill_rewards_msg(
        config.fee_collector_addr.into_string(),
        creation_fee,
    )?);

    let asset_decimals_vec = asset_infos
        .iter()
        .map(|asset| {
            asset
                .query_decimals(env.contract.address.clone(), &deps.querier)
                .unwrap()
        })
        .collect::<Vec<_>>();
    // Check if the asset infos are the same
    if asset_infos
        .iter()
        .any(|asset| asset_infos.iter().filter(|&a| a == asset).count() > 1)
    {
        return Err(ContractError::SameAsset {});
    }

    // Verify pool fees
    pool_fees.is_valid()?;

    let pair_id = PAIR_COUNTER.load(deps.storage)?;
    // if no identifier is provided, use the vault counter (id) as identifier
    // TODO: Review, do we really want this or just use the pair_id? Pair_id is simple u64 values while identifier is a string
    let identifier = pair_identifier.unwrap_or(pair_id.to_string());

    // check if there is an existing vault with the given identifier
    let pair = get_pair_by_identifier(&deps.as_ref(), identifier.clone());
    if pair.is_ok() {
        return Err(ContractError::PairExists {
            asset_infos: asset_infos
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            identifier,
        });
    }

    // prepare labels for creating the pair token with a meaningful name
    let pair_label = asset_infos
        .iter()
        .map(|asset| asset.to_owned().get_label(&deps.as_ref()))
        .collect::<Result<Vec<_>, _>>()?
        .join("-");

    let mut attributes = Vec::<Attribute>::new();

    // Convert all asset_infos into assets with 0 balances
    let assets = asset_infos
        .iter()
        .map(|asset_info| Asset {
            info: asset_info.clone(),
            amount: Uint128::zero(),
        })
        .collect::<Vec<_>>();

    #[allow(unreachable_code)]
    let pair_creation_msg = if token_factory_lp {
        #[cfg(all(
            not(feature = "token_factory"),
            not(feature = "osmosis_token_factory"),
            not(feature = "injective")
        ))]
        return Err(ContractError::TokenFactoryNotEnabled {});
        let lp_symbol = format!("{pair_label}.vault.{identifier}.{LP_SYMBOL}");
        let denom = format!("{}/{}/{}", "factory", env.contract.address, lp_symbol);
        let lp_asset = AssetInfo::NativeToken { denom };

        PAIRS.save(
            deps.storage,
            identifier.clone(),
            &PairInfo {
                asset_infos: asset_infos.clone(),
                pair_type: pair_type.clone(),
                liquidity_token: lp_asset.clone(),
                asset_decimals: asset_decimals_vec,
                pool_fees,
                assets,
                balances: vec![Uint128::zero(); asset_infos.len()],
            },
        )?;

        attributes.push(attr("lp_asset", lp_asset.to_string()));

        #[cfg(any(
            feature = "token_factory",
            feature = "osmosis_token_factory",
            feature = "injective"
        ))]
        Ok(white_whale_std::tokenfactory::create_denom::create_denom(
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
            pair_label,
            identifier,
            info.sender.into_string(),
            env.block.height
        );
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

        PAIRS.save(
            deps.storage,
            identifier.clone(),
            &PairInfo {
                asset_infos: asset_infos.clone(),
                pair_type: pair_type.clone(),
                liquidity_token: lp_asset.clone(),
                asset_decimals: asset_decimals_vec,
                assets,
                pool_fees,
                balances: vec![Uint128::zero(); asset_infos.len()],
            },
        )?;

        attributes.push(attr("lp_asset", lp_asset.to_string()));

        let lp_token_name = format!("{pair_label}-LP");

        Ok::<CosmosMsg, ContractError>(CosmosMsg::Wasm(WasmMsg::Instantiate2 {
            admin: None,
            code_id,
            label: lp_token_name.to_owned(),
            msg: to_json_binary(&TokenInstantiateMsg {
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
    attributes.push(attr("pair_identifier", identifier.as_str()));

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
