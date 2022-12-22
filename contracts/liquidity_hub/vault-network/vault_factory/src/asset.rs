use cosmwasm_std::{StdError, StdResult};

use terraswap::asset::AssetInfo;

pub trait AssetReference {
    fn get_reference(&self) -> &[u8];
}

impl AssetReference for AssetInfo {
    fn get_reference(&self) -> &[u8] {
        // convert AssetInfo into vec bytes
        match self {
            AssetInfo::Token { contract_addr } => contract_addr.as_bytes(),
            AssetInfo::NativeToken { denom } => denom.as_bytes(),
        }
    }
}

pub fn from_reference(reference: Vec<u8>) -> StdResult<AssetInfo> {
    // try to parse reference as a contract address
    if let Ok(contract_addr) = String::from_utf8(reference.clone()) {
        return Ok(AssetInfo::Token { contract_addr });
    }

    // try to parse reference as a denom value
    if let Ok(denom) = String::from_utf8(reference) {
        return Ok(AssetInfo::NativeToken { denom });
    }

    // if reference could not be parsed as either a contract address or denom value, return an error
    Err(StdError::generic_err("Invalid reference value".to_string()))
}
