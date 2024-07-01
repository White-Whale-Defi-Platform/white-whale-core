use crate::epoch_manager::epoch_manager::Epoch;
use std::fmt::Display;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    to_json_binary, Addr, Coin, CosmosMsg, Decimal, StdResult, Timestamp, Uint128, WasmMsg,
};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

#[cw_serde]
pub struct Config {
    /// Pool Manager contract address for swapping
    pub pool_manager_addr: Addr,
    /// Epoch Manager contract address
    pub epoch_manager_addr: Addr,
    /// Distribution denom for the rewards
    pub distribution_denom: String,
    /// Unbonding period in nanoseconds. The time that needs to pass before an unbonded position can
    /// be withdrawn
    pub unbonding_period: u64,
    /// A fraction that controls the effect of time on the weight of a bond. If the growth rate is set
    /// to zero, time will have no impact on the weight.
    pub growth_rate: Decimal,
    /// Denom of the asset to be bonded. Can't only be set at instantiation.
    pub bonding_assets: Vec<String>,
    /// Grace period the maximum age of a reward bucket before it's considered expired and fees
    /// are forwarded from it
    pub grace_period: u64,
}

#[cw_serde]
#[derive(Default)]
pub struct RewardBucket {
    // id of the reward bucket. Matches the epoch id it's associated with
    pub id: u64,
    // Epoch start time
    pub epoch_start_time: Timestamp,
    // Initial fees to be distributed in this reward bucket.
    pub total: Vec<Coin>,
    // Fees left to be claimed on this reward bucket. These available fees are forwarded when the
    // reward bucket expires.
    pub available: Vec<Coin>,
    // Fees that were claimed on this reward bucket. For keeping record on the total fees claimed.
    pub claimed: Vec<Coin>,
    // Global index snapshot taken at the time of reward bucket creation
    pub global_index: GlobalIndex,
}

#[cw_serde]
#[derive(Default)]
pub struct UpcomingRewardBucket {
    // The fees to be distributed in this reward bucket.
    pub total: Vec<Coin>,
}

#[cw_serde]
pub struct Bond {
    /// The id of the bond.
    pub id: u64,
    /// The epoch id at which the Bond was created.
    pub created_at_epoch: u64,
    /// The epoch id at which the bond was last time updated.
    pub last_updated: u64,
    /// The amount of bonded tokens.
    pub asset: Coin,
    /// The weight of the bond at the given block height.
    pub weight: Uint128,
    /// The time at which the Bond was unbonded.
    pub unbonded_at: Option<u64>,
    /// The owner of the bond.
    pub receiver: Addr,
}

impl Default for Bond {
    fn default() -> Self {
        Self {
            id: 0,
            asset: Coin {
                denom: String::new(),
                amount: Uint128::zero(),
            },
            created_at_epoch: Default::default(),
            unbonded_at: None,
            last_updated: Default::default(),
            weight: Uint128::zero(),
            receiver: Addr::unchecked(""),
        }
    }
}

#[cw_serde]
#[derive(Default)]
pub struct GlobalIndex {
    /// The epoch id the global index was taken a snapshot for
    pub epoch_id: u64,
    /// The total amount of tokens bonded in the contract.
    pub bonded_amount: Uint128,
    /// Assets that are bonded in the contract.
    pub bonded_assets: Vec<Coin>,
    /// The epoch id at which the total bond was updated.
    pub last_updated: u64,
    /// The total weight of the contract at the given updated_last epoch id.
    pub last_weight: Uint128,
}

#[cw_serde]
pub struct InstantiateMsg {
    /// Denom to be swapped to and rewarded
    pub distribution_denom: String,
    /// Unbonding period in epochs. The time (in epochs) that needs to pass before an unbonded position can
    /// be withdrawn
    pub unbonding_period: u64,
    /// Weight grow rate. Needs to be between 0 and 1.
    pub growth_rate: Decimal,
    /// [String] denoms of the assets that can be bonded.
    pub bonding_assets: Vec<String>,
    /// Grace period the maximum age of a reward bucket before it's considered expired and fees
    /// are forwarded from it
    pub grace_period: u64,
    /// The epoch manager contract
    pub epoch_manager_addr: String,
}

#[cw_serde]
pub struct EpochChangedHookMsg {
    pub current_epoch: Epoch,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    /// Bonds the specified [Asset].
    Bond,
    /// Unbonds the specified [Asset].
    Unbond {
        /// The asset to unbond.
        asset: Coin,
    },
    /// Sends withdrawable assets of the given denom to the user. An asset becomes withdrawable after
    /// it has been unbonded and the unbonding period has passed.
    Withdraw {
        /// The denom to withdraw.
        denom: String,
    },
    /// Updates the [Config] of the contract.
    UpdateConfig {
        /// The new epoch manager address.
        epoch_manager_addr: Option<String>,
        /// The new pool manager address.
        pool_manager_addr: Option<String>,
        /// The unbonding period.
        unbonding_period: Option<u64>,
        /// The new growth rate.
        growth_rate: Option<Decimal>,
    },
    /// Claims the available rewards
    Claim,
    /// Claims the available rewards on behalf of the specified address. Only executable by the contract.
    ClaimForAddr { address: String },
    /// Fills the contract with new rewards.
    FillRewards,
    /// Epoch Changed hook implementation. Creates a new reward bucket for the rewards flowing from
    /// this time on, i.e. to be distributed in the upcoming epoch. Also, forwards the expiring
    /// reward bucket (only 21 of them are live at a given moment)
    EpochChangedHook {
        /// The current epoch, the one that was newly created.
        current_epoch: Epoch,
    },
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the [Config] of te contract.
    #[returns(Config)]
    Config,
    /// Returns the amount of assets that have been bonded by the specified address.
    #[returns(BondedResponse)]
    Bonded {
        /// The address to check for bonded assets. If none is provided, all bonded assets in the
        /// contract are returned.
        address: Option<String>,
    },
    /// Returns the amount of tokens of the given denom that are been unbonded by the specified address.
    /// Allows pagination with start_after and limit.
    #[returns(UnbondingResponse)]
    Unbonding {
        /// The address to check for unbonding assets.
        address: String,
        /// The denom to check for unbonding assets.
        denom: String,
        /// The amount of unbonding assets to skip. Allows pagination.
        start_after: Option<u64>,
        /// The maximum amount of unbonding assets to return.
        limit: Option<u8>,
    },
    /// Returns the amount of unbonding tokens of the given denom for the specified address that can
    /// be withdrawn, i.e. that have passed the unbonding period.
    #[returns(WithdrawableResponse)]
    Withdrawable {
        /// The address to check for withdrawable assets.
        address: String,
        /// The denom to check for withdrawable assets.
        denom: String,
    },
    /// Returns the global index of the contract.
    #[returns(GlobalIndex)]
    GlobalIndex {
        /// The reward bucket id to check for the global index. If none is provided, the current global index
        /// is returned.
        reward_bucket_id: Option<u64>,
    },
    /// Returns the [RewardBucket]s that can be claimed by an address.
    #[returns(ClaimableRewardBucketsResponse)]
    Claimable {
        /// The address to check for claimable reward buckets. If none is provided, all possible
        /// reward buckets stored in the contract that can potentially be claimed are returned.
        address: Option<String>,
    },
    /// Returns the rewards for the given address.
    #[returns(RewardsResponse)]
    Rewards { address: String },
}

#[cw_serde]
pub struct MigrateMsg {}

/// Response for the Rewards query
#[cw_serde]
pub struct RewardsResponse {
    /// The rewards that can be claimed by the address.
    pub rewards: Vec<Coin>,
}

/// Response for the Bonded query
#[cw_serde]
pub struct BondedResponse {
    /// The total amount of bonded tokens by the address. Bear in mind the bonded assets are
    /// considered to be equal for this purpose.
    pub total_bonded: Uint128,
    /// The assets that are bonded by the address.
    pub bonded_assets: Vec<Coin>,
}

/// Response for the Unbonding query
#[cw_serde]
pub struct UnbondingResponse {
    /// The total amount of unbonded tokens by the address.
    pub total_amount: Uint128,
    /// The total amount of unbonded assets by the address.
    pub unbonding_requests: Vec<Bond>,
}

/// Response for the Withdrawable query
#[cw_serde]
pub struct WithdrawableResponse {
    /// The total amount of withdrawable assets by the address.
    pub withdrawable_amount: Uint128,
}

/// Response for the Claimable query
#[cw_serde]
pub struct ClaimableRewardBucketsResponse {
    /// The reward buckets that can be claimed by the address.
    pub reward_buckets: Vec<RewardBucket>,
}

#[cw_serde]
pub struct TemporalBondAction {
    pub sender: Addr,
    pub coin: Coin,
    pub action: BondAction,
}

#[cw_serde]
pub enum BondAction {
    Bond,
    Unbond,
}

impl Display for BondAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BondAction::Bond => write!(f, "bond"),
            BondAction::Unbond => write!(f, "unbond"),
        }
    }
}

/// Creates a message to fill rewards on the whale lair contract.
pub fn fill_rewards_msg(contract_addr: String, assets: Vec<Coin>) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr,
        msg: to_json_binary(&ExecuteMsg::FillRewards)?,
        funds: assets,
    }))
}
