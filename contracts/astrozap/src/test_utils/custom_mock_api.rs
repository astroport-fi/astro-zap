use cosmwasm_std::testing::MockApi;
use cosmwasm_std::{
    Addr, Api, CanonicalAddr, RecoverPubkeyError, StdError, StdResult, VerificationError,
};

#[derive(Default, Copy, Clone)]
pub struct CustomMockApi(MockApi);

impl Api for CustomMockApi {
    /// For this mock API, we consider a predefined list of strings to be valid addresses, and all
    /// others invalid
    fn addr_validate(&self, human: &str) -> StdResult<Addr> {
        let valid_addresses = vec![
            "astro_token",
            "bluna_token",
            "luna_ust_pair",
            "luna_ust_lp_token",
            "astro_ust_pair",
            "astro_ust_lp_token",
            "bluna_luna_pair",
            "bluna_luna_lp_token",
        ];
        if valid_addresses.contains(&human) {
            self.0.addr_validate(human)
        } else {
            Err(StdError::generic_err(format!("[mock]: invalid address: {}", human)))
        }
    }

    // For all other functions, we simply dispatch to parent

    fn addr_canonicalize(&self, human: &str) -> StdResult<CanonicalAddr> {
        self.0.addr_canonicalize(human)
    }

    fn addr_humanize(&self, canonical: &CanonicalAddr) -> StdResult<Addr> {
        self.0.addr_humanize(canonical)
    }

    fn secp256k1_verify(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        self.0.secp256k1_verify(message_hash, signature, public_key)
    }

    fn secp256k1_recover_pubkey(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        recovery_param: u8,
    ) -> Result<Vec<u8>, RecoverPubkeyError> {
        self.0
            .secp256k1_recover_pubkey(message_hash, signature, recovery_param)
    }

    fn ed25519_verify(
        &self,
        message: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        self.0.ed25519_verify(message, signature, public_key)
    }

    fn ed25519_batch_verify(
        &self,
        messages: &[&[u8]],
        signatures: &[&[u8]],
        public_keys: &[&[u8]],
    ) -> Result<bool, VerificationError> {
        self.0
            .ed25519_batch_verify(messages, signatures, public_keys)
    }

    fn debug(&self, message: &str) {
        self.0.debug(message)
    }
}
