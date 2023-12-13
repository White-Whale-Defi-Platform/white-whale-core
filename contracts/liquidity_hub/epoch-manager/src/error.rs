use cosmwasm_std::StdError;
use cw_controllers::{AdminError, HookError};
use semver::Version;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    AdminError(#[from] AdminError),

    #[error("{0}")]
    HookError(#[from] HookError),

    #[error("The epoch id has overflowed.")]
    EpochOverflow,

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("Attempt to migrate to version {new_version}, but contract is on a higher version {current_version}")]
    MigrateInvalidVersion {
        new_version: Version,
        current_version: Version,
    },

    #[error("The current epoch epoch has not expired yet.")]
    CurrentEpochNotExpired,

    #[error("start_time must be in the future.")]
    InvalidStartTime,

    #[error("genesis_epoch must be equal to start_epoch.start_time.")]
    EpochConfigMismatch,

    #[error("No epoch found with id {0}.")]
    NoEpochFound(u64),
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}
