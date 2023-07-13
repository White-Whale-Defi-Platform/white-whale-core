use std::fmt;
use std::fmt::Display;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Timestamp, Uint128, Uint64};

#[cw_serde]
pub struct InstantiateMsg {
    pub start_epoch: EpochV2,
    pub epoch_config: EpochConfig,
}

#[cw_serde]
pub enum ExecuteMsg {
    CreateEpoch {},
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
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct Config {
    pub epoch_config: EpochConfig,
}

pub type ConfigResponse = Config;

#[cw_serde]
pub struct EpochResponse {
    pub epoch: EpochV2,
}

#[cw_serde]
pub struct ClaimableEpochsResponse {
    pub epochs: Vec<EpochV2>,
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
pub struct EpochV2 {
    // Epoch identifier
    pub id: u64,
    // Epoch start time
    pub start_time: Timestamp,
}

impl Display for EpochV2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Epoch {{ id: {}, start_time: {} }}",
            self.id, self.start_time,
        )
    }
}
