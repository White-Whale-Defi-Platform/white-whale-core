use cosmwasm_std::{DivideByZeroError, OverflowError, StdError, Uint64};
use cw_utils::ParseReplyError;
use semver::Version;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("Can't handle the given reply id: {0}")]
    UnknownReplyId(u64),

    #[error("Invalid grace period: {0}. Must be between 1 and 10.")]
    InvalidGracePeriod(Uint64),

    #[error("Invalid epoch duration: {0}. Must be at least 1 day.")]
    InvalidEpochDuration(Uint64),

    #[error("The assets sent don't match the assets expected.")]
    AssetMismatch {},

    #[error("There are no claimable fees.")]
    NothingToClaim {},

    #[error("The rewards cannot exceed the available claimable fees.")]
    InvalidReward {},

    #[error("The current epoch epoch has not expired yet.")]
    CurrentEpochNotExpired {},

    #[error("Couldn't read data for new epoch.")]
    CannotReadEpoch {},

    #[error("Can't refill the epoch with id {0}. Either it hasn't been started or it has already been claimed.")]
    CannotRefillEpoch(Uint64),

    #[error("{0}")]
    DivideByZeroError(#[from] DivideByZeroError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("{0}")]
    ParseReplyError(#[from] ParseReplyError),

    #[error("Attempt to migrate to version {new_version}, but contract is on a higher version {current_version}")]
    MigrateInvalidVersion {
        new_version: Version,
        current_version: Version,
    },
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}
