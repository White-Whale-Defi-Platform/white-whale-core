use std::collections::HashMap;

use cosmwasm_std::{BankMsg, Coin, CosmosMsg, StdError, StdResult, Uint128};

#[cfg(feature = "injective")]
pub const PEGGY_PREFIX: &str = "peggy";
#[cfg(feature = "injective")]
const PEGGY_ADDR_SIZE: usize = 47usize;
#[cfg(feature = "injective")]
const PEGGY_ADDR_TAKE: usize = 3usize;
pub const IBC_PREFIX: &str = "ibc";
pub const FACTORY_PREFIX: &str = "factory";
const FACTORY_SUBDENOM_SIZE: usize = 44usize;
const FACTORY_PATH_TAKE: usize = 3usize;
const IBC_HASH_TAKE: usize = 4usize;
const IBC_HASH_SIZE: usize = 64usize;

pub fn get_label(denom: &str) -> StdResult<String> {
    #[cfg(feature = "injective")]
    {
        if is_ethereum_bridged_asset(denom) {
            return get_ethereum_bridged_asset_label(denom);
        }
    }
    if is_ibc_token(denom) {
        get_ibc_token_label(denom)
    } else if is_factory_token(denom) {
        get_factory_token_label(denom)
    } else {
        Ok(denom.to_owned())
    }
}

#[cfg(feature = "injective")]
/// Verifies if the given denom is an Ethereum bridged asset on Injective.
fn is_ethereum_bridged_asset(denom: &str) -> bool {
    denom.starts_with(PEGGY_PREFIX) && denom.len() == PEGGY_ADDR_SIZE
}

#[cfg(feature = "injective")]
/// Builds the label for an Ethereum bridged asset denom in such way that it returns a label like "peggy0x123..456".
/// Call after [is_ethereum_bridged_asset] has been successful
fn get_ethereum_bridged_asset_label(denom: &str) -> StdResult<String> {
    let ethereum_asset_prefix = format!("{}{}", PEGGY_PREFIX, "0x");
    let mut asset_address = denom
        .strip_prefix(ethereum_asset_prefix.as_str())
        .ok_or_else(|| StdError::generic_err("Splitting ethereum bridged asset denom failed"))?
        .to_string();

    asset_address.drain(PEGGY_ADDR_TAKE..asset_address.len() - PEGGY_ADDR_TAKE);
    asset_address.insert_str(PEGGY_ADDR_TAKE, "...");
    asset_address.insert_str(0, ethereum_asset_prefix.as_str());

    Ok(asset_address)
}

/// Verifies if the given denom is an ibc token or not
pub fn is_ibc_token(denom: &str) -> bool {
    let split: Vec<&str> = denom.splitn(2, '/').collect();

    if split[0] == IBC_PREFIX && split.len() == 2 {
        return split[1].matches(char::is_alphanumeric).count() == IBC_HASH_SIZE;
    }

    false
}

/// Builds the label for an ibc token denom in such way that it returns a label like "ibc/1234...5678".
/// Call after [is_ibc_token] has been successful
fn get_ibc_token_label(denom: &str) -> StdResult<String> {
    let ibc_token_prefix = format!("{}{}", IBC_PREFIX, '/');
    let mut token_hash = denom
        .strip_prefix(ibc_token_prefix.as_str())
        .ok_or_else(|| StdError::generic_err("Splitting ibc token denom failed"))?
        .to_string();

    token_hash.drain(IBC_HASH_TAKE..token_hash.len() - IBC_HASH_TAKE);
    token_hash.insert_str(IBC_HASH_TAKE, "...");
    token_hash.insert_str(0, ibc_token_prefix.as_str());

    Ok(token_hash)
}

/// Verifies if the given denom is a factory token or not.
/// A factory token has the following structure: factory/{creating contract address}/{Subdenom}
/// Subdenom can be of length at most 44 characters, in [0-9a-zA-Z./].
pub fn is_factory_token(denom: &str) -> bool {
    let split: Vec<&str> = denom.splitn(3, '/').collect();

    if split.len() < 3 && split[0] != FACTORY_PREFIX {
        return false;
    }

    if split.len() > 3 {
        let merged = split[3..].join("/");
        if merged.len() > FACTORY_SUBDENOM_SIZE {
            return false;
        }
    }

    true
}
/// Verifies if the given denom is a factory token or not.
/// A factory token has the following structure: factory/{creating contract address}/{Subdenom}
/// Subdenom can be of length at most 44 characters, in [0-9a-zA-Z./].
pub fn is_native_lp_token(denom: &str) -> bool {
    let split: Vec<&str> = denom.splitn(3, '/').collect();

    if split.len() < 3 && split[0] != FACTORY_PREFIX {
        return false;
    }

    if split.len() > 3 {
        let merged = split[3..].join("/");
        if merged.len() > FACTORY_SUBDENOM_SIZE {
            return false;
        }
    }

    true
}

/// Gets the subdenom of a factory token. To be called after [is_factory_token] has been successful.
pub fn get_factory_token_subdenom(denom: &str) -> StdResult<&str> {
    let subdenom = denom.splitn(3, '/').nth(2);

    subdenom.map_or_else(
        || {
            Err(StdError::generic_err(
                "Splitting factory token subdenom failed",
            ))
        },
        Ok,
    )
}

/// Builds the label for a factory token denom in such way that it returns a label like "factory/mig...xyz/123...456".
/// Call after [crate::pool_network::asset::is_factory_token] has been successful
fn get_factory_token_label(denom: &str) -> StdResult<String> {
    let factory_token_prefix = format!("{}{}", FACTORY_PREFIX, '/');
    let factory_path: Vec<&str> = denom
        .strip_prefix(factory_token_prefix.as_str())
        .ok_or_else(|| StdError::generic_err("Splitting factory token path failed"))?
        .splitn(2, '/')
        .collect();

    let mut token_creator = factory_path[0].to_string();
    let mut token_subdenom = factory_path[1].to_string();

    token_creator.drain(FACTORY_PATH_TAKE..token_creator.len() - FACTORY_PATH_TAKE);
    token_creator.insert_str(FACTORY_PATH_TAKE, "...");

    if token_subdenom.len() > 2 * FACTORY_PATH_TAKE {
        token_subdenom.drain(FACTORY_PATH_TAKE..token_subdenom.len() - FACTORY_PATH_TAKE);
        token_subdenom.insert_str(FACTORY_PATH_TAKE, "...");
    }

    Ok(format!("{FACTORY_PREFIX}/{token_creator}/{token_subdenom}"))
}

/// Deducts the coins in `to_deduct` from `coins` if they exist.
pub fn deduct_coins(coins: Vec<Coin>, to_deduct: Vec<Coin>) -> StdResult<Vec<Coin>> {
    let mut updated_coins = coins.to_vec();

    for coin in to_deduct {
        if let Some(existing_coin) = updated_coins.iter_mut().find(|c| c.denom == coin.denom) {
            existing_coin.amount = existing_coin.amount.checked_sub(coin.amount)?;
        } else {
            return Err(StdError::generic_err(format!(
                "Error: Cannot deduct {} {}. Coin not found.",
                coin.amount, coin.denom
            )));
        }
    }

    updated_coins.retain(|coin| coin.amount > Uint128::zero());

    Ok(updated_coins)
}
/// Aggregates coins from two vectors, summing up the amounts of coins that are the same.
pub fn aggregate_coins(coins: Vec<Coin>) -> StdResult<Vec<Coin>> {
    let mut aggregation_map: HashMap<String, Uint128> = HashMap::new();

    // aggregate coins by denom
    for coin in coins {
        if let Some(existing_amount) = aggregation_map.get_mut(&coin.denom) {
            *existing_amount = existing_amount.checked_add(coin.amount)?;
        } else {
            aggregation_map.insert(coin.denom.clone(), coin.amount);
        }
    }

    // create a new vector from the aggregation map
    let mut aggregated_coins: Vec<Coin> = Vec::new();
    for (denom, amount) in aggregation_map {
        aggregated_coins.push(Coin { denom, amount });
    }

    Ok(aggregated_coins)
}

/// Creates a CosmosMsg::Bank::BankMsg::Burn message with the given coin.
pub fn burn_coin_msg(coin: Coin) -> CosmosMsg {
    CosmosMsg::Bank(BankMsg::Burn { amount: vec![coin] })
}
