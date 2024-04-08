use cosmwasm_std::{
    attr, Attribute, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, Uint128,
};
use white_whale_std::{
    pool_network::{asset::PairType, pair::PoolFee, querier::query_native_decimals},
    whale_lair::fill_rewards_msg,
};

use crate::state::{add_allow_native_token, get_pair_by_identifier, PAIR_COUNTER};
use crate::{
    state::{Config, MANAGER_CONFIG, PAIRS},
    ContractError,
};

use white_whale_std::pool_manager::PairInfo;
use white_whale_std::pool_network::querier::query_balance;

pub const MAX_ASSETS_PER_POOL: usize = 4;
pub const LP_SYMBOL: &str = "uLP";

/// Creates a liquidity pool pair with 2, 3, or N assets. The function dynamically handles different numbers of assets,
/// allowing for the creation of pairs with varying configurations. The maximum number of assets per pool is defined by
/// the constant `MAX_ASSETS_PER_POOL`.
///
/// # Example
///
/// ```rust
/// # use cosmwasm_std::{DepsMut, Decimal, Env, MessageInfo, Response, CosmosMsg, WasmMsg, to_json_binary};
/// # use white_whale_std::pool_network::{asset::{PairType}, pair::PoolFee};
/// # use white_whale_std::fee::Fee;
/// # use pool_manager::error::ContractError;
/// # use pool_manager::manager::commands::MAX_ASSETS_PER_POOL;
/// # use pool_manager::manager::commands::create_pair;
/// # use std::convert::TryInto;
/// #
/// # fn example(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
/// let asset_infos = vec![
///     "uatom".into(),
///     "uscrt".into(),
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
/// let response = create_pair(deps, env, info, asset_infos, pool_fees, pair_type, None)?;
/// # Ok(response)
/// # }
/// ```
#[allow(unreachable_code)]
#[allow(clippy::too_many_arguments)]
pub fn create_pair(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset_denoms: Vec<String>, //Review just a vec<asset>
    pool_fees: PoolFee,
    pair_type: PairType,
    pair_identifier: Option<String>,
) -> Result<Response, ContractError> {
    // Load config for pool creation fee
    let config: Config = MANAGER_CONFIG.load(deps.storage)?;

    // Check if fee was provided and is sufficient
    if !config.pool_creation_fee.amount.is_zero() {
        // verify fee payment
        let amount = cw_utils::must_pay(&info, &config.pool_creation_fee.denom)?;
        if amount != config.pool_creation_fee.amount {
            return Err(ContractError::InvalidPairCreationFee {
                amount,
                expected: config.pool_creation_fee.amount,
            });
        }
    }

    // Prepare the sending of pair creation fee
    let mut messages: Vec<CosmosMsg> = vec![];

    // send pool creation fee to whale lair
    let creation_fee = vec![config.pool_creation_fee];

    // send pair creation fee to whale lair i.e the new fee_collector
    messages.push(fill_rewards_msg(
        config.whale_lair_addr.into_string(),
        creation_fee,
    )?);

    let asset_decimals_vec = asset_denoms
        .iter()
        .map(|asset| {
            //todo pass the asset_decimals in the create_pair msg. Let the user creating the pool
            // defining the decimals, they are incentivized to do it right as they are paying a fee

            let _ = query_native_decimals(
                &deps.querier,
                env.contract.address.clone(),
                asset.to_string(),
            );

            0u8
        })
        .collect::<Vec<u8>>();

    // Check if the asset infos are the same
    if asset_denoms
        .iter()
        .any(|asset| asset_denoms.iter().filter(|&a| a == asset).count() > 1)
    {
        return Err(ContractError::SameAsset {});
    }

    // Verify pool fees
    pool_fees.is_valid()?;

    let pair_id = PAIR_COUNTER.load(deps.storage)?;
    // if no identifier is provided, use the pool counter (id) as identifier
    let identifier = pair_identifier.unwrap_or(pair_id.to_string());

    // check if there is an existing pool with the given identifier
    let pair = get_pair_by_identifier(&deps.as_ref(), &identifier);
    if pair.is_ok() {
        return Err(ContractError::PairExists {
            asset_infos: asset_denoms
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            identifier,
        });
    }

    // prepare labels for creating the pair token with a meaningful name
    let pair_label = asset_denoms.join("-");

    let mut attributes = Vec::<Attribute>::new();

    // Convert all asset_infos into assets with 0 balances
    let assets = asset_denoms
        .iter()
        .map(|asset_info| Coin {
            denom: asset_info.clone(),
            amount: Uint128::zero(),
        })
        .collect::<Vec<_>>();

    let lp_symbol = format!("{pair_label}.pool.{identifier}.{LP_SYMBOL}");
    let lp_asset = format!("{}/{}/{}", "factory", env.contract.address, lp_symbol);

    PAIRS.save(
        deps.storage,
        &identifier,
        &PairInfo {
            asset_denoms: asset_denoms.clone(),
            pair_type: pair_type.clone(),
            lp_denom: lp_asset.clone(),
            asset_decimals: asset_decimals_vec,
            pool_fees,
            assets,
        },
    )?;

    attributes.push(attr("lp_asset", lp_asset));

    #[cfg(all(
        not(feature = "token_factory"),
        not(feature = "osmosis_token_factory"),
        not(feature = "injective")
    ))]
    {
        return Err(ContractError::TokenFactoryNotEnabled {});
    }

    messages.push(white_whale_std::tokenfactory::create_denom::create_denom(
        env.contract.address,
        lp_symbol,
    ));

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
