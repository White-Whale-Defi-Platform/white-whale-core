use crate::pool_network::asset::{Asset, AssetInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Timestamp, Uint64};
use std::fmt;
use std::fmt::Display;

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub bonding_contract_addr: Addr,
    pub fee_collector_addr: Addr,
    pub grace_period: Uint64,
    pub epoch_config: EpochConfig,
    pub distribution_asset: AssetInfo,
}

#[cw_serde]
#[derive(Default)]
pub struct Epoch {
    // Epoch identifier
    pub id: Uint64,
    // Epoch start time
    pub start_time: Timestamp,
    // Initial fees to be distributed in this epoch.
    pub total: Vec<Asset>,
    // Fees left to be claimed on this epoch. These available fees are forwarded when the epoch expires.
    pub available: Vec<Asset>,
    // Fees that were claimed on this epoch. For keeping record on the total fees claimed.
    pub claimed: Vec<Asset>,
}

impl Display for Epoch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Epoch {{ id: {}, start_time: {}, total: {:?}, available: {:?}, claimed: {:?} }}",
            self.id, self.start_time, self.total, self.available, self.claimed
        )
    }
}

#[cw_serde]
pub struct EpochConfig {
    /// The duration of an epoch in nanoseconds.
    pub duration: Uint64,
    /// Timestamp for the first epoch, in nanoseconds.
    pub genesis_epoch: Uint64,
}

impl Display for EpochConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EpochConfig {{ epoch_duration: {}, genesis_epoch: {}, }}",
            self.duration, self.genesis_epoch
        )
    }
}

#[cw_serde]
pub struct InstantiateMsg {
    /// Address of the bonding contract.
    pub bonding_contract_addr: String,
    /// Fee collector address.
    pub fee_collector_addr: String,
    /// The duration of the grace period in epochs, i.e. how many expired epochs can be claimed
    /// back in time after new epochs have been created.
    pub grace_period: Uint64,
    /// Configuration for the epoch.
    pub epoch_config: EpochConfig,
    /// The asset that is going to be distributed by the contracdt.
    pub distribution_asset: AssetInfo,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Creates a new epoch, forwarding available tokens from epochs that are past the grace period.
    /// Can only be executed by the fee collector.
    NewEpoch {},

    /// Claims tokens from the current epoch and all epochs that are in the grace period.
    /// Sends all tokens to the sender.
    Claim {},

    /// Updates the [Config] of the contract.
    UpdateConfig {
        owner: Option<String>,
        bonding_contract_addr: Option<String>,
        fee_collector_addr: Option<String>,
        grace_period: Option<Uint64>,
        distribution_asset: Option<AssetInfo>,
        epoch_config: Option<EpochConfig>,
    },

    /// Sets the last claimed epoch for the given address. This is only used the very first time
    /// a user bonds tokens in the whale lair, to ensure the user cannot claim rewards from
    /// past epochs
    SetLastClaimedEpoch { address: String, epoch_id: Uint64 },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the current epoch, which is the last on the EPOCHS map.
    #[returns(Config)]
    Config {},

    /// Returns the current epoch, which is the last on the EPOCHS map.
    #[returns(EpochResponse)]
    CurrentEpoch {},

    /// Returns the epoch with the given id.
    #[returns(EpochResponse)]
    Epoch { id: Uint64 },

    /// Returns the [Epoch]s that can be claimed.
    #[returns(ClaimableEpochsResponse)]
    ClaimableEpochs {},

    /// Returns the [Epoch]s that can be claimed by an address.
    #[returns(ClaimableEpochsResponse)]
    Claimable { address: String },

    /// Returns the [Epoch]s that can be claimed by an address.
    #[returns(EpochResponse)]
    LastClaimedEpoch { address: String },
}

#[cw_serde]
pub struct EpochResponse {
    pub epoch: Epoch,
}

#[cw_serde]
pub struct ClaimableEpochsResponse {
    pub epochs: Vec<Epoch>,
}

#[cw_serde]
pub struct LastClaimedEpochResponse {
    pub address: Addr,
    pub last_claimed_epoch_id: Uint64,
}

#[cw_serde]
pub struct MigrateMsg {}
