use cosmwasm_std::{
    Addr, Api, CanonicalAddr, RecoverPubkeyError, StdError, StdResult, VerificationError,
};
// Reworked mock api to work with instantiate2 in mock_querier, can eventually be removed
#[derive(Copy, Clone, Default)]
pub struct MockSimpleApi {}

impl Api for MockSimpleApi {
    fn addr_validate(&self, input: &str) -> StdResult<Addr> {
        let canonical = self.addr_canonicalize(input)?;
        let normalized = self.addr_humanize(&canonical)?;
        if input != normalized && normalized != "contract1" {
            return Err(StdError::generic_err(
                "Invalid input: address not normalized",
            ));
        }

        Ok(Addr::unchecked(input))
    }

    fn addr_canonicalize(&self, input: &str) -> StdResult<CanonicalAddr> {
        // Very straigthfoward canonicalization, we simply serialize the address to bytes
        Ok(input.chars().map(|c| c as u8).collect::<Vec<_>>().into())
    }

    fn addr_humanize(&self, canonical: &CanonicalAddr) -> StdResult<Addr> {
        let mut address: String = canonical.0 .0.iter().map(|&c| c as char).collect();
        if address
            == "\u{82}³r\u{13}Ø\r¯ËÌB\u{85}Ó^-b¸\u{19}\u{89}Z\rBðf0ç\u{9d}µís+æ\u{16}".to_string()
        {
            address = "contract1".to_string();
        }
        Ok(Addr::unchecked(address))
    }

    fn secp256k1_verify(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        cosmwasm_std::testing::MockApi::default().secp256k1_verify(
            message_hash,
            signature,
            public_key,
        )
    }

    fn secp256k1_recover_pubkey(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        recovery_param: u8,
    ) -> Result<Vec<u8>, RecoverPubkeyError> {
        cosmwasm_std::testing::MockApi::default().secp256k1_recover_pubkey(
            message_hash,
            signature,
            recovery_param,
        )
    }

    fn ed25519_verify(
        &self,
        message: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        cosmwasm_std::testing::MockApi::default().ed25519_verify(message, signature, public_key)
    }

    fn ed25519_batch_verify(
        &self,
        messages: &[&[u8]],
        signatures: &[&[u8]],
        public_keys: &[&[u8]],
    ) -> Result<bool, VerificationError> {
        cosmwasm_std::testing::MockApi::default().ed25519_batch_verify(
            messages,
            signatures,
            public_keys,
        )
    }

    fn debug(&self, message: &str) {
        println!("{}", message);
    }
}