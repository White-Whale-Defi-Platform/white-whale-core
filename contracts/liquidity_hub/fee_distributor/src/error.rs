use cosmwasm_std::{DivideByZeroError, OverflowError, StdError, Uint64};
use cw_utils::ParseReplyError;
use semver::Version;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
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

    #[error("Invalid epoch duration: {0}. Must be at least 1 day, in nanoseconds.")]
    InvalidEpochDuration(Uint64),

    #[error("The assets sent don't match the assets expected.")]
    AssetMismatch {},

    #[error("There are no claimable rewards.")]
    NothingToClaim {},

    #[error("The rewards cannot exceed the available claimable fees.")]
    InvalidReward {},

    #[error("The current epoch epoch has not expired yet.")]
    CurrentEpochNotExpired {},

    #[error("The genesis epoch is set to start in the future, query the config for more details.")]
    GenesisEpochNotStarted {},

    #[error("Couldn't read data for new epoch.")]
    CannotReadEpoch {},

    #[error("Can't lower the grace period.")]
    GracePeriodDecrease {},

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
