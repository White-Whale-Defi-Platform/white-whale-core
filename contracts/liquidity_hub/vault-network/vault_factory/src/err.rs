use cosmwasm_std::Addr;
use semver::Version;
use thiserror::Error;

pub type StdResult<T> = Result<T, VaultFactoryError>;

#[derive(Error, Debug, PartialEq)]
pub enum VaultFactoryError {
    #[error("{0}")]
    Std(#[from] cosmwasm_std::StdError),

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("This asset already has a vault at \"{addr}\"")]
    ExistingVault { addr: Addr },

    #[error("Attempt to migrate to version {new_version}, but contract is on a higher version {current_version}")]
    MigrateInvalidVersion {
        new_version: Version,
        current_version: Version,
    },

    #[error("Vault doesn't exist given the vault address provided")]
    NonExistentVault {},
}

impl From<semver::Error> for VaultFactoryError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}
