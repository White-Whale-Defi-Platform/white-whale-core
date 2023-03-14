use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Timestamp, Uint128, Uint64};
use std::fmt;

use terraswap::asset::Asset;

use crate::state::{Config, Epoch};

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
}

#[cw_serde]
pub struct EpochConfig {
    /// The duration of an epoch in seconds.
    pub duration: Uint64,
    /// Timestamp for the midnight when the first epoch is going to be created.
    pub genesis_epoch: Uint64,
}

impl fmt::Display for EpochConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EpochConfig {{ epoch_duration: {}, genesis_epoch: {}, }}",
            self.duration, self.genesis_epoch
        )
    }
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
        staking_contract_addr: Option<String>,
        fee_collector_addr: Option<String>,
        grace_period: Option<Uint64>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the current epoch, which is the last on the EPOCHS map.
    #[returns(Config)]
    Config {},

    /// Returns the current epoch, which is the last on the EPOCHS map.
    #[returns(Epoch)]
    CurrentEpoch {},

    /// Returns the epoch with the given id.
    #[returns(Option<Epoch>)]
    Epoch { id: u128 },

    /// Returns the [Epoch]s that can be claimed.
    #[returns(Vec<Epoch>)]
    ClaimableEpochs {},
}
