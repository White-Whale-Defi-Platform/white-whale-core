use crate::{
    epoch_manager::epoch_manager::EpochV2,
    pool_network::asset::{Asset, AssetInfo, ToCoins},
};

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    to_json_binary, Addr, Coin, CosmosMsg, Decimal, StdResult, Timestamp, Uint128, Uint64, WasmMsg,
};

#[cw_serde]
pub struct Config {
    /// Owner of the contract.
    pub owner: Addr,
    /// Unbonding period in nanoseconds.
    pub unbonding_period: Uint64,
    /// A fraction that controls the effect of time on the weight of a bond. If the growth rate is set
    /// to zero, time will have no impact on the weight.
    pub growth_rate: Decimal,
    /// Denom of the asset to be bonded. Can't only be set at instantiation.
    pub bonding_assets: Vec<AssetInfo>,
    /// Address of the fee distributor contract.
    pub fee_distributor_addr: Addr,
    /// The duration of the grace period in epochs, i.e. how many expired epochs can be claimed
    pub grace_period: Uint64,
}

#[cw_serde]
#[derive(Default)]
pub struct Epoch {
    // Epoch identifier
    pub id: Uint64,
    // Epoch start time
    pub start_time: Timestamp,
    // Initial fees to be distributed in this epoch.
    pub total: Vec<Coin>,
    // Fees left to be claimed on this epoch. These available fees are forwarded when the epoch expires.
    pub available: Vec<Coin>,
    // Fees that were claimed on this epoch. For keeping record on the total fees claimed.
    pub claimed: Vec<Coin>,
    // Global index taken at the time of Epoch Creation
    pub global_index: GlobalIndex,
}

#[cw_serde]
pub struct Bond {
    /// The amount of bonded tokens.
    pub asset: Coin,
    /// The timestamp at which the bond was done.
    pub timestamp: Timestamp,
    /// The weight of the bond at the given block height.
    pub weight: Uint128,
}

impl Default for Bond {
    fn default() -> Self {
        Self {
            asset: Coin {
                denom: String::new(),
                amount: Uint128::zero(),
            },
            timestamp: Timestamp::default(),
            weight: Uint128::zero(),
        }
    }
}

#[cw_serde]
#[derive(Default)]
pub struct GlobalIndex {
    /// The total amount of tokens bonded in the contract.
    pub bonded_amount: Uint128,
    /// Assets that are bonded in the contract.
    pub bonded_assets: Vec<Coin>,
    /// The timestamp at which the total bond was registered.
    pub timestamp: Timestamp,
    /// The total weight of the bond at the given block height.
    pub weight: Uint128,
}

#[cw_serde]
pub struct InstantiateMsg {
    /// Unbonding period in nanoseconds.
    pub unbonding_period: Uint64,
    /// Weight grow rate. Needs to be between 0 and 1.
    pub growth_rate: Decimal,
    /// [AssetInfo] of the assets that can be bonded.
    pub bonding_assets: Vec<AssetInfo>,
}

#[cw_serde]
pub struct EpochChangedHookMsg {
    pub current_epoch: EpochV2,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Bonds the specified [Asset].
    Bond {
        asset: Coin,
    },
    /// Unbonds the specified [Asset].
    Unbond {
        asset: Coin,
    },
    /// Sends withdrawable unbonded tokens to the user.
    Withdraw {
        denom: String,
    },
    /// Updates the [Config] of the contract.
    UpdateConfig {
        owner: Option<String>,
        unbonding_period: Option<Uint64>,
        growth_rate: Option<Decimal>,
        fee_distributor_addr: Option<String>,
    },
    Claim {},

    /// V2 MESSAGES

    /// Fills the whale lair with new rewards.
    FillRewards {
        assets: Vec<Coin>,
    },

    /// Creates a new bucket for the rewards flowing from this time on, i.e. to be distributed in the next epoch. Also, forwards the expiring epoch (only 21 epochs are live at a given moment)
    EpochChangedHook {
        msg: EpochChangedHookMsg,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the [Config] of te contract.
    #[returns(Config)]
    Config {},

    /// Returns the amount of assets that have been bonded by the specified address.
    #[returns(BondedResponse)]
    Bonded { address: String },

    /// Returns the amount of tokens of the given denom that are been unbonded by the specified address.
    /// Allows pagination with start_after and limit.
    #[returns(UnbondingResponse)]
    Unbonding {
        address: String,
        denom: String,
        start_after: Option<u64>,
        limit: Option<u8>,
    },

    /// Returns the amount of unbonding tokens of the given denom for the specified address that can
    /// be withdrawn, i.e. that have passed the unbonding period.
    #[returns(WithdrawableResponse)]
    Withdrawable { address: String, denom: String },

    /// Returns the weight of the address.
    #[returns(BondingWeightResponse)]
    Weight {
        address: String,
        timestamp: Option<Timestamp>,
        global_index: Option<GlobalIndex>,
    },

    /// Returns the total amount of assets that have been bonded to the contract.
    #[returns(BondedResponse)]
    TotalBonded {},

    /// Returns the global index of the contract.
    #[returns(GlobalIndex)]
    GlobalIndex {},
}

#[cw_serde]
pub struct MigrateMsg {}

/// Response for the Bonded query
#[cw_serde]
pub struct BondedResponse {
    pub total_bonded: Uint128,
    pub bonded_assets: Vec<Coin>,
    pub first_bonded_epoch_id: Uint64,
}

/// Response for the Unbonding query
#[cw_serde]
pub struct UnbondingResponse {
    pub total_amount: Uint128,
    pub unbonding_requests: Vec<Bond>,
}

/// Response for the Withdrawable query
#[cw_serde]
pub struct WithdrawableResponse {
    pub withdrawable_amount: Uint128,
}

/// Response for the Weight query.
#[cw_serde]
pub struct BondingWeightResponse {
    pub address: String,
    pub weight: Uint128,
    pub global_weight: Uint128,
    pub share: Decimal,
    pub timestamp: Timestamp,
}

/// Creates a message to fill rewards on the whale lair contract.
pub fn fill_rewards_msg(contract_addr: String, assets: Vec<Asset>) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr,
        msg: to_json_binary(&ExecuteMsg::FillRewards {
            assets: assets.to_coins()?,
        })?,
        funds: assets.to_coins()?,
    }))
}

#[cw_serde]
pub struct EpochResponse {
    pub epoch: Epoch,
}

#[cw_serde]
pub struct ClaimableEpochsResponse {
    pub epochs: Vec<Epoch>,
}
