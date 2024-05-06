#![allow(clippy::module_inception)]
use std::fmt;
use std::fmt::Display;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Timestamp, Uint64};
use cw_controllers::HooksResponse;

#[cw_serde]
pub struct InstantiateMsg {
    pub start_epoch: Epoch,
    pub epoch_config: EpochConfig,
}

#[cw_serde]
pub enum ExecuteMsg {
    CreateEpoch,
    AddHook {
        contract_addr: String,
    },
    RemoveHook {
        contract_addr: String,
    },
    UpdateConfig {
        owner: Option<String>,
        epoch_config: Option<EpochConfig>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the current epoch, which is the last on the EPOCHS map.
    #[returns(ConfigResponse)]
    Config {},

    /// Returns the current epoch, which is the last on the EPOCHS map.
    #[returns(EpochResponse)]
    CurrentEpoch {},

    /// Returns the epoch with the given id.
    #[returns(EpochResponse)]
    Epoch { id: u64 },

    /// Returns the hooks in the registry.
    #[returns(HooksResponse)]
    Hooks {},

    /// Returns whether or not a hook has been registered.
    #[returns(bool)]
    Hook { hook: String },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct Config {
    pub epoch_config: EpochConfig,
}

impl Config {
    pub fn to_config_response(self, owner: Addr) -> ConfigResponse {
        ConfigResponse {
            owner,
            epoch_config: self.epoch_config,
        }
    }
}

#[cw_serde]
pub struct ConfigResponse {
    pub owner: Addr,
    pub epoch_config: EpochConfig,
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
#[derive(Default)]
pub struct Epoch {
    // Epoch identifier
    pub id: u64,
    // Epoch start time
    pub start_time: Timestamp,
}

impl Epoch {
    pub fn to_epoch_response(self) -> EpochResponse {
        EpochResponse { epoch: self }
    }
}

impl Display for Epoch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Epoch {{ id: {}, start_time: {} }}",
            self.id, self.start_time,
        )
    }
}
