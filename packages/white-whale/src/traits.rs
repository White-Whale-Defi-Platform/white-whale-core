use crate::pool_network::asset::AssetInfo;
use cosmwasm_std::Decimal;

/// A trait for converting an Option<Decimal> to a string.
pub trait OptionDecimal {
    fn to_string(self) -> String;
}

impl OptionDecimal for Option<Decimal> {
    fn to_string(self) -> String {
        match self {
            None => "None".to_string(),
            Some(d) => d.to_string(),
        }
    }
}

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
