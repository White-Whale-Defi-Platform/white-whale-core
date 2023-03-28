use cosmwasm_std::StdError;
use semver::Version;
use thiserror::Error;
use white_whale::pool_network::asset::AssetInfo;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("Attempt to migrate to version {new_version}, but contract is on a higher version {current_version}")]
    MigrateInvalidVersion {
        new_version: Version,
        current_version: Version,
    },

    #[error("max_concurrent_flows cannot be set to zero")]
    UnspecifiedConcurrentFlows,

    #[error(
        "Attempt to create a duplicate incentive contract. Incentive already exists at {incentive}"
    )]
    DuplicateIncentiveContract { incentive: AssetInfo },

    #[error("Error callback from incentive contract: {reason}")]
    CreateIncentiveCallback { reason: String },

    #[error("Sender is not authorized to invoke functions on the incentive factory")]
    Unauthorized,

    #[error("Unknown reply id {id}")]
    UnknownReplyId { id: u64 },

    #[error("Invalid unbonding range, specified min as {min} and max as {max}")]
    InvalidUnbondingRange {
        /// The minimum unbonding time
        min: u64,
        /// The maximum unbonding time
        max: u64,
    },
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}
