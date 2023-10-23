use std::fmt;
use std::fmt::{Display, Formatter};

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    coins, to_binary, Addr, Api, BankMsg, CanonicalAddr, Coin, CosmosMsg, Deps, MessageInfo,
    QuerierWrapper, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;

use crate::pool_network::querier::{
    query_balance, query_native_decimals, query_token_balance, query_token_info,
};

pub const MINIMUM_LIQUIDITY_AMOUNT: Uint128 = Uint128::new(1_000u128);
const IBC_HASH_TAKE: usize = 4usize;
const IBC_HASH_SIZE: usize = 64usize;
pub const IBC_PREFIX: &str = "ibc";
pub const FACTORY_PREFIX: &str = "factory";
const FACTORY_SUBDENOM_SIZE: usize = 44usize;
const FACTORY_PATH_TAKE: usize = 3usize;

#[cfg(feature = "injective")]
pub const PEGGY_PREFIX: &str = "peggy";
#[cfg(feature = "injective")]
const PEGGY_ADDR_SIZE: usize = 47usize;
#[cfg(feature = "injective")]
const PEGGY_ADDR_TAKE: usize = 3usize;

#[cw_serde]
pub struct Asset {
    pub info: AssetInfo,
    pub amount: Uint128,
}

impl Display for Asset {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}{}", self.amount, self.info)
    }
}

impl Asset {
    pub fn is_native_token(&self) -> bool {
        self.info.is_native_token()
    }

    pub fn into_msg(self, recipient: Addr) -> StdResult<CosmosMsg> {
        let amount = self.amount;

        match &self.info {
            AssetInfo::Token { contract_addr } => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: recipient.to_string(),
                    amount,
                })?,
                funds: vec![],
            })),
            AssetInfo::NativeToken { denom } => Ok(CosmosMsg::Bank(BankMsg::Send {
                to_address: recipient.to_string(),
                amount: vec![Coin {
                    amount: self.amount,
                    denom: denom.to_string(),
                }],
            })),
        }
    }

    pub fn into_submsg(self, recipient: Addr) -> StdResult<SubMsg> {
        Ok(SubMsg::new(self.into_msg(recipient)?))
    }

    pub fn into_burn_msg(self) -> StdResult<CosmosMsg> {
        let burn_msg = match self.info {
            AssetInfo::Token { contract_addr } => CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&Cw20ExecuteMsg::Burn {
                    amount: self.amount,
                })?,
                funds: vec![],
            }),
            AssetInfo::NativeToken { denom } => CosmosMsg::Bank(BankMsg::Burn {
                amount: coins(self.amount.u128(), denom),
            }),
        };

        Ok(burn_msg)
    }

    pub fn assert_sent_native_token_balance(&self, message_info: &MessageInfo) -> StdResult<()> {
        if let AssetInfo::NativeToken { denom } = &self.info {
            match message_info.funds.iter().find(|x| x.denom == *denom) {
                Some(coin) => {
                    if self.amount == coin.amount {
                        Ok(())
                    } else {
                        Err(StdError::generic_err("Native token balance mismatch between the argument and the transferred"))
                    }
                }
                None => {
                    if self.amount.is_zero() {
                        Ok(())
                    } else {
                        Err(StdError::generic_err("Native token balance mismatch between the argument and the transferred"))
                    }
                }
            }
        } else {
            Ok(())
        }
    }

    pub fn to_raw(&self, api: &dyn Api) -> StdResult<AssetRaw> {
        Ok(AssetRaw {
            info: match &self.info {
                AssetInfo::NativeToken { denom } => AssetInfoRaw::NativeToken {
                    denom: denom.to_string(),
                },
                AssetInfo::Token { contract_addr } => AssetInfoRaw::Token {
                    contract_addr: api.addr_canonicalize(contract_addr.as_str())?,
                },
            },
            amount: self.amount,
        })
    }

    /// Gets an asset id, i.e. either denom or contract_addr. Used by the pair contract when withdrawing
    /// liquidity to subtract the fees collected by the protocol for a given Asset laying on the pool
    pub fn get_id(self) -> String {
        match self.info {
            AssetInfo::Token { contract_addr } => contract_addr,
            AssetInfo::NativeToken { denom } => denom,
        }
    }
}

/// AssetInfo contract_addr is usually passed from the cw20 hook
/// so we can trust the contract_addr is properly validated.
#[derive(PartialOrd)]
#[cw_serde]
pub enum AssetInfo {
    Token { contract_addr: String },
    NativeToken { denom: String },
}

impl Display for AssetInfo {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            AssetInfo::NativeToken { denom } => write!(f, "{denom}"),
            AssetInfo::Token { contract_addr } => write!(f, "{contract_addr}"),
        }
    }
}

impl AssetInfo {
    pub fn to_raw(&self, api: &dyn Api) -> StdResult<AssetInfoRaw> {
        match self {
            AssetInfo::NativeToken { denom } => Ok(AssetInfoRaw::NativeToken {
                denom: denom.to_string(),
            }),
            AssetInfo::Token { contract_addr } => Ok(AssetInfoRaw::Token {
                contract_addr: api.addr_canonicalize(contract_addr.as_str())?,
            }),
        }
    }

    pub fn is_native_token(&self) -> bool {
        match self {
            AssetInfo::NativeToken { .. } => true,
            AssetInfo::Token { .. } => false,
        }
    }
    pub fn query_balance(
        &self,
        querier: &QuerierWrapper,
        api: &dyn Api,
        addr: Addr,
    ) -> StdResult<Uint128> {
        match self {
            AssetInfo::Token { contract_addr, .. } => {
                query_token_balance(querier, api.addr_validate(contract_addr.as_str())?, addr)
            }
            AssetInfo::NativeToken { denom, .. } => query_balance(querier, addr, denom.to_string()),
        }
    }

    pub fn equal(&self, asset: &AssetInfo) -> bool {
        match self {
            AssetInfo::Token { contract_addr, .. } => {
                let self_contract_addr = contract_addr;
                match asset {
                    AssetInfo::Token { contract_addr, .. } => self_contract_addr == contract_addr,
                    AssetInfo::NativeToken { .. } => false,
                }
            }
            AssetInfo::NativeToken { denom, .. } => {
                let self_denom = denom;
                match asset {
                    AssetInfo::Token { .. } => false,
                    AssetInfo::NativeToken { denom, .. } => self_denom == denom,
                }
            }
        }
    }

    pub fn query_decimals(&self, account_addr: Addr, querier: &QuerierWrapper) -> StdResult<u8> {
        match self {
            AssetInfo::NativeToken { denom } => {
                query_native_decimals(querier, account_addr, denom.to_string())
            }
            AssetInfo::Token { contract_addr } => {
                let token_info = query_token_info(querier, Addr::unchecked(contract_addr))?;
                Ok(token_info.decimals)
            }
        }
    }

    /// Gets an asset label, used by the factory to create pool pairs and lp tokens with custom names
    pub fn get_label(self, deps: &Deps) -> StdResult<String> {
        match self {
            AssetInfo::Token { contract_addr } => Ok(query_token_info(
                &deps.querier,
                deps.api.addr_validate(contract_addr.as_str())?,
            )?
            .symbol),
            AssetInfo::NativeToken { denom } => {
                #[cfg(feature = "injective")]
                {
                    if is_ethereum_bridged_asset(&denom) {
                        return get_ethereum_bridged_asset_label(denom.clone());
                    }
                }
                if is_ibc_token(&denom) {
                    get_ibc_token_label(denom)
                } else if is_factory_token(&denom) {
                    get_factory_token_label(denom)
                } else {
                    Ok(denom)
                }
            }
        }
    }

    pub fn validate(self, deps: &Deps) -> StdResult<AssetInfo> {
        match self.clone() {
            AssetInfo::NativeToken { .. } => {}
            AssetInfo::Token { contract_addr } => {
                deps.api.addr_validate(contract_addr.as_str())?;
            }
        }

        Ok(self)
    }
}

/// Verifies if there's a factory token in the vector of [AssetInfo]s.
pub fn has_factory_token(assets: &[AssetInfo]) -> bool {
    assets.iter().any(|asset| match asset {
        AssetInfo::Token { .. } => false,
        AssetInfo::NativeToken { denom } => is_factory_token(denom),
    })
}

/// Verifies if the given denom is an ibc token or not
fn is_ibc_token(denom: &str) -> bool {
    let split: Vec<&str> = denom.splitn(2, '/').collect();

    if split[0] == IBC_PREFIX && split.len() == 2 {
        return split[1].matches(char::is_alphanumeric).count() == IBC_HASH_SIZE;
    }

    false
}

/// Builds the label for an ibc token denom in such way that it returns a label like "ibc/1234...5678".
/// Call after [is_ibc_token] has been successful
fn get_ibc_token_label(denom: String) -> StdResult<String> {
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

/// Builds the label for a factory token denom in such way that it returns a label like "factory/mig...xyz/123...456".
/// Call after [is_factory_token] has been successful
fn get_factory_token_label(denom: String) -> StdResult<String> {
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

#[cfg(feature = "injective")]
/// Verifies if the given denom is an Ethereum bridged asset on Injective.
fn is_ethereum_bridged_asset(denom: &str) -> bool {
    denom.starts_with(PEGGY_PREFIX) && denom.len() == PEGGY_ADDR_SIZE
}

#[cfg(feature = "injective")]
/// Builds the label for an Ethereum bridged asset denom in such way that it returns a label like "peggy0x123..456".
/// Call after [is_ethereum_bridged_asset] has been successful
fn get_ethereum_bridged_asset_label(denom: String) -> StdResult<String> {
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

#[cw_serde]
pub struct AssetRaw {
    pub info: AssetInfoRaw,
    pub amount: Uint128,
}

impl AssetRaw {
    pub fn to_normal(&self, api: &dyn Api) -> StdResult<Asset> {
        Ok(Asset {
            info: match &self.info {
                AssetInfoRaw::NativeToken { denom } => AssetInfo::NativeToken {
                    denom: denom.to_string(),
                },
                AssetInfoRaw::Token { contract_addr } => AssetInfo::Token {
                    contract_addr: api.addr_humanize(contract_addr)?.to_string(),
                },
            },
            amount: self.amount,
        })
    }
}

#[cw_serde]
pub enum AssetInfoRaw {
    Token { contract_addr: CanonicalAddr },
    NativeToken { denom: String },
}

impl AssetInfoRaw {
    pub fn to_normal(&self, api: &dyn Api) -> StdResult<AssetInfo> {
        match self {
            AssetInfoRaw::NativeToken { denom } => Ok(AssetInfo::NativeToken {
                denom: denom.to_string(),
            }),
            AssetInfoRaw::Token { contract_addr } => Ok(AssetInfo::Token {
                contract_addr: api.addr_humanize(contract_addr)?.to_string(),
            }),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            AssetInfoRaw::NativeToken { denom } => denom.as_bytes(),
            AssetInfoRaw::Token { contract_addr } => contract_addr.as_slice(),
        }
    }

    pub fn equal(&self, asset: &AssetInfoRaw) -> bool {
        match self {
            AssetInfoRaw::Token { contract_addr, .. } => {
                let self_contract_addr = contract_addr;
                match asset {
                    AssetInfoRaw::Token { contract_addr, .. } => {
                        self_contract_addr == contract_addr
                    }
                    AssetInfoRaw::NativeToken { .. } => false,
                }
            }
            AssetInfoRaw::NativeToken { denom, .. } => {
                let self_denom = denom;
                match asset {
                    AssetInfoRaw::Token { .. } => false,
                    AssetInfoRaw::NativeToken { denom, .. } => self_denom == denom,
                }
            }
        }
    }
}

// We define a custom struct for each query response
#[cw_serde]
pub struct PairInfo {
    pub asset_infos: [AssetInfo; 2],
    pub contract_addr: String,
    pub liquidity_token: AssetInfo,
    pub asset_decimals: [u8; 2],
    pub pair_type: PairType,
}

#[cw_serde]
pub struct PairInfoRaw {
    pub asset_infos: [AssetInfoRaw; 2],
    pub contract_addr: CanonicalAddr,
    pub liquidity_token: AssetInfoRaw,
    pub asset_decimals: [u8; 2],
    pub pair_type: PairType,
}

impl PairInfoRaw {
    pub fn to_normal(&self, api: &dyn Api) -> StdResult<PairInfo> {
        Ok(PairInfo {
            liquidity_token: self.liquidity_token.to_normal(api)?,
            contract_addr: api.addr_humanize(&self.contract_addr)?.to_string(),
            asset_infos: [
                self.asset_infos[0].to_normal(api)?,
                self.asset_infos[1].to_normal(api)?,
            ],
            asset_decimals: self.asset_decimals,
            pair_type: self.pair_type.to_owned(),
        })
    }

    pub fn query_pools(
        &self,
        querier: &QuerierWrapper,
        api: &dyn Api,
        contract_addr: Addr,
    ) -> StdResult<[Asset; 2]> {
        let info_0: AssetInfo = self.asset_infos[0].to_normal(api)?;
        let info_1: AssetInfo = self.asset_infos[1].to_normal(api)?;
        Ok([
            Asset {
                amount: info_0.query_balance(querier, api, contract_addr.clone())?,
                info: info_0,
            },
            Asset {
                amount: info_1.query_balance(querier, api, contract_addr)?,
                info: info_1,
            },
        ])
    }
}

#[cw_serde]
pub enum PairType {
    StableSwap {
        /// The amount of amplification to perform on the constant product part of the swap formula.
        amp: u64,
    },
    ConstantProduct,
}

impl PairType {
    /// Gets a string representation of the pair type
    pub fn get_label(&self) -> &str {
        match self {
            PairType::ConstantProduct => "ConstantProduct",
            PairType::StableSwap { .. } => "StableSwap",
        }
    }
}

pub trait ToCoins {
    fn to_coins(&self) -> StdResult<Vec<Coin>>;
}

impl ToCoins for Vec<Asset> {
    /// converts a Vec<Asset> into a Vec<Coin>
    fn to_coins(&self) -> StdResult<Vec<Coin>> {
        self.iter()
            .map(|asset| {
                let denom = match &asset.info {
                    AssetInfo::Token { .. } => {
                        return Err(StdError::generic_err("Not a native token."));
                    }
                    AssetInfo::NativeToken { denom } => denom,
                };

                Ok(Coin {
                    denom: denom.to_string(),
                    amount: asset.amount,
                })
            })
            .collect()
    }
}

/// Aggregates assets from two vectors, summing up the amounts of assets that are the same.
pub fn aggregate_assets(assets: Vec<Asset>, other_assets: Vec<Asset>) -> StdResult<Vec<Asset>> {
    let mut aggregated_assets: Vec<Asset> = Vec::with_capacity(assets.len() + other_assets.len());
    for asset in assets {
        aggregated_assets.push(asset.clone());
    }

    for asset in other_assets {
        if let Some(existing_asset) = aggregated_assets.iter_mut().find(|a| a.info == asset.info) {
            existing_asset.amount = existing_asset.amount.checked_add(asset.amount)?;
        } else {
            aggregated_assets.push(asset.clone());
        }
    }

    Ok(aggregated_assets)
}

/// Deducts assets from two vectors, subtracting the amounts of assets that are the same.
pub fn deduct_assets(assets: Vec<Asset>, to_deduct: Vec<Asset>) -> StdResult<Vec<Asset>> {
    let mut updated_assets = assets.to_vec();

    for asset in to_deduct {
        if let Some(existing_asset) = updated_assets.iter_mut().find(|a| a.info == asset.info) {
            existing_asset.amount = existing_asset.amount.checked_sub(asset.amount)?;
        } else {
            return Err(StdError::generic_err(format!(
                "Error: Cannot deduct {} {}. Asset not found.",
                asset.amount, asset.info
            )));
        }
    }

    Ok(updated_assets)
}

#[cw_serde]
pub struct TrioInfo {
    pub asset_infos: [AssetInfo; 3],
    pub contract_addr: String,
    pub liquidity_token: AssetInfo,
    pub asset_decimals: [u8; 3],
}

#[cw_serde]
pub struct TrioInfoRaw {
    pub asset_infos: [AssetInfoRaw; 3],
    pub contract_addr: CanonicalAddr,
    pub liquidity_token: AssetInfoRaw,
    pub asset_decimals: [u8; 3],
}

impl TrioInfoRaw {
    pub fn to_normal(&self, api: &dyn Api) -> StdResult<TrioInfo> {
        Ok(TrioInfo {
            liquidity_token: self.liquidity_token.to_normal(api)?,
            contract_addr: api.addr_humanize(&self.contract_addr)?.to_string(),
            asset_infos: [
                self.asset_infos[0].to_normal(api)?,
                self.asset_infos[1].to_normal(api)?,
                self.asset_infos[2].to_normal(api)?,
            ],
            asset_decimals: self.asset_decimals,
        })
    }

    pub fn query_pools(
        &self,
        querier: &QuerierWrapper,
        api: &dyn Api,
        contract_addr: Addr,
    ) -> StdResult<[Asset; 3]> {
        let info_0: AssetInfo = self.asset_infos[0].to_normal(api)?;
        let info_1: AssetInfo = self.asset_infos[1].to_normal(api)?;
        let info_2: AssetInfo = self.asset_infos[2].to_normal(api)?;
        Ok([
            Asset {
                amount: info_0.query_balance(querier, api, contract_addr.clone())?,
                info: info_0,
            },
            Asset {
                amount: info_1.query_balance(querier, api, contract_addr.clone())?,
                info: info_1,
            },
            Asset {
                amount: info_2.query_balance(querier, api, contract_addr)?,
                info: info_2,
            },
        ])
    }
}

/// Gets the total supply of the given liquidity asset
pub fn get_total_share(deps: &Deps, liquidity_asset: String) -> StdResult<Uint128> {
    #[cfg(any(
        feature = "token_factory",
        feature = "osmosis_token_factory",
        feature = "injective"
    ))]
    let total_share = if is_factory_token(liquidity_asset.as_str()) {
        //bank query total
        deps.querier.query_supply(&liquidity_asset)?.amount
    } else {
        query_token_info(
            &deps.querier,
            deps.api.addr_validate(liquidity_asset.as_str())?,
        )?
        .total_supply
    };
    #[cfg(all(
        not(feature = "token_factory"),
        not(feature = "osmosis_token_factory"),
        not(feature = "injective")
    ))]
    let total_share = query_token_info(
        &deps.querier,
        deps.api.addr_validate(liquidity_asset.as_str())?,
    )?
    .total_supply;

    Ok(total_share)
}
