use cosmwasm_std::Uint128;
use pool_network::asset::Asset;
use semver::Version;
use thiserror::Error;

pub type StdResult<T> = Result<T, VaultRouterError>;

#[derive(Error, Debug, PartialEq)]
pub enum VaultRouterError {
    #[error("{0}")]
    Std(#[from] cosmwasm_std::StdError),

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Attempt to migrate to version {new_version}, but contract is on a higher version {current_version}")]
    MigrateInvalidVersion {
        new_version: Version,
        current_version: Version,
    },

    #[error("Attempt to flash-loan asset \"{asset}\" that does not have a vault")]
    InvalidAsset { asset: Asset },

    #[error("Negative profits when attempting to flash-loan asset \"{input}\" (got {output_amount}, needed {required_amount})")]
    NegativeProfit {
        input: Asset,
        output_amount: Uint128,
        required_amount: Uint128,
    },

    #[error("Nested flash-loans are disabled")]
    NestedFlashLoansDisabled {},
}

impl From<semver::Error> for VaultRouterError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}
