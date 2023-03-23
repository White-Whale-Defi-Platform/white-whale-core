use cosmwasm_std::StdError;
use cw_utils::ParseReplyError;
use semver::Version;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Can't aggregate fees provided specific contracts")]
    InvalidContractsFeeAggregation {},

    #[error("Attempt to migrate to version {new_version}, but contract is on a higher version {current_version}")]
    MigrateInvalidVersion {
        new_version: Version,
        current_version: Version,
    },

    #[error("Couldn't read data for new epoch.")]
    CannotReadEpoch {},

    #[error("{0}")]
    ParseReplyError(#[from] ParseReplyError),

    #[error("Can't handle the given reply id: {0}")]
    UnknownReplyId(u64),
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}
