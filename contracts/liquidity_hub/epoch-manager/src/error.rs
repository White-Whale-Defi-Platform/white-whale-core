use cosmwasm_std::StdError;
use cw_controllers::{AdminError, HookError};
use cw_migrate_error_derive::cw_migrate_invalid_version_error;
use cw_utils::PaymentError;
use thiserror::Error;

#[cw_migrate_invalid_version_error]
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

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("The current epoch epoch has not expired yet.")]
    CurrentEpochNotExpired,

    #[error("The genesis epoch has not started yet.")]
    GenesisEpochHasNotStarted,

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
