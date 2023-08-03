use cosmwasm_std::{Addr, Uint128};
use semver::Version;
use thiserror::Error;
use white_whale::pool_network::asset::AssetInfo;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] cosmwasm_std::StdError),

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("The asset \"{asset_info}\" already has a vault")]
    ExistingVault { asset_info: AssetInfo },

    #[error("Attempt to migrate to version {new_version}, but contract is on a higher version {current_version}")]
    MigrateInvalidVersion {
        new_version: Version,
        current_version: Version,
    },

    #[error("Vault doesn't exist given the vault address provided")]
    NonExistentVault {},

    #[error("Invalid vault creation fee paid. Received {amount}, expected {expected}")]
    InvalidVaultCreationFee { amount: Uint128, expected: Uint128 },
}


impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}
