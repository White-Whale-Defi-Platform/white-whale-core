use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Decimal, Deps};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, UniqueIndex};

pub use white_whale_std::pool_manager::Config;
use white_whale_std::pool_manager::{PoolInfo, SwapOperation};

use crate::ContractError;

/// Holds information about the single side liquidity provision temporarily until the swap/liquidity
/// provision is completed
#[cw_serde]
pub struct SingleSideLiquidityProvisionBuffer {
    /// The receiver of the LP
    pub receiver: String,
    /// The expected offer asset balance in the contract after the single side liquidity provision
    /// is done. Used for validations.
    pub expected_offer_asset_balance_in_contract: Coin,
    /// The expected ask asset balance in the contract after the single side liquidity provision
    /// is done. Used for validations.
    pub expected_ask_asset_balance_in_contract: Coin,
    /// Half of the offer asset, i.e. the amount of the offer asset that is going to be swapped
    /// for the ask asset so the LP is provided in balanced proportions.
    pub offer_asset_half: Coin,
    /// The expected ask asset after half of the offer asset is swapped for the ask asset. This is
    /// computed via a swap simulation.
    pub expected_ask_asset: Coin,
    /// The remaining data for the liquidity provision.
    pub liquidity_provision_data: LiquidityProvisionData,
}

/// Holds information about the intended liquidity provision when a user provides liquidity with a
/// single asset.
#[cw_serde]
pub struct LiquidityProvisionData {
    /// The maximum allowable spread between the bid and ask prices for the pool.
    /// When provided, if the spread exceeds this value, the liquidity provision will not be
    /// executed.
    pub max_spread: Option<Decimal>,
    /// A percentage value representing the acceptable slippage for the operation.
    /// When provided, if the slippage exceeds this value, the liquidity provision will not be
    /// executed.
    pub slippage_tolerance: Option<Decimal>,
    /// The identifier for the pool to provide liquidity for.
    pub pool_identifier: String,
    /// The amount of time in seconds to unlock tokens if taking part on the incentives. If not passed,
    /// the tokens will not be locked and the LP tokens will be returned to the user.
    pub unlocking_duration: Option<u64>,
    /// The identifier of the position to lock the LP tokens in the incentive manager, if any.
    pub lock_position_identifier: Option<String>,
}

pub const SINGLE_SIDE_LIQUIDITY_PROVISION_BUFFER: Item<SingleSideLiquidityProvisionBuffer> =
    Item::new("single_side_liquidity_provision_buffer");

pub const POOLS: IndexedMap<&str, PoolInfo, PoolIndexes> = IndexedMap::new(
    "pools",
    PoolIndexes {
        lp_asset: UniqueIndex::new(|v| v.lp_denom.to_string(), "pools__lp_asset"),
    },
);

pub struct PoolIndexes<'a> {
    pub lp_asset: UniqueIndex<'a, String, PoolInfo, String>,
}

impl<'a> IndexList<PoolInfo> for PoolIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<PoolInfo>> + '_> {
        let v: Vec<&dyn Index<PoolInfo>> = vec![&self.lp_asset];
        Box::new(v.into_iter())
    }
}

/// Gets the pool given its identifier
pub fn get_pool_by_identifier(
    deps: &Deps,
    pool_identifier: &str,
) -> Result<PoolInfo, ContractError> {
    POOLS
        .may_load(deps.storage, pool_identifier)?
        .ok_or(ContractError::UnExistingPool)
}

/// Swap routes are used to establish defined routes for a given fee
/// token to a desired fee token and is used for fee collection
#[cw_serde]
pub struct SwapOperations {
    /// creator of the swap route, can remove it later
    pub creator: String,
    /// The operations to be executed for a given swap.
    pub swap_operations: Vec<SwapOperation>,
}

pub const SWAP_ROUTES: Map<(&str, &str), SwapOperations> = Map::new("swap_routes");

pub const CONFIG: Item<Config> = Item::new("config");
pub const POOL_COUNTER: Item<u64> = Item::new("pool_count");
