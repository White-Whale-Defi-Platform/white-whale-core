use pool_network::asset::AssetInfo;

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
