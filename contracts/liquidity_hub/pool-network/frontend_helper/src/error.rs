use cosmwasm_std::{StdError, Uint128};
use semver::Version;
use thiserror::Error;
use white_whale::pool_network::asset::Asset;

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

    #[error("Failed to deposit due to {reason}")]
    DepositCallback { reason: String },

    #[error("Sender is not authorized to invoke functions on the frontend helper")]
    Unauthorized {},

    #[error("Pair had no incentive associated with it")]
    MissingIncentive { pair_address: String },

    #[error("Unknown reply id {id}")]
    UnknownReplyId { id: u64 },

    #[error(
        "Token {asset} did not have allowance set to high enough, only had {current_allowance} provided"
    )]
    MissingToken {
        asset: Asset,
        current_allowance: Uint128,
    },
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}
