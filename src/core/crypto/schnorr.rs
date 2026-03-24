use k256::schnorr::{SigningKey, VerifyingKey, Signature};
use k256::ecdsa::signature::{Signer, Verifier};
use rand::rngs::OsRng;
use crate::core::crypto::hash::Hash;

pub struct KeyPairWrapper {
    signing_key: SigningKey,
}

impl KeyPairWrapper {
    pub fn new() -> Self {
        let signing_key = SigningKey::random(&mut OsRng);
        Self { signing_key }
    }

    pub fn public_key(&self) -> VerifyingKey {
        *self.signing_key.verifying_key()
    }

    pub fn sign(&self, msg: &[u8]) -> Signature {
        let msg_hash = Hash::new(msg);
        self.signing_key.sign(&msg_hash.as_bytes()[..])
    }
}

pub fn verify(pubkey: &VerifyingKey, msg: &[u8], signature: &Signature) -> bool {
    let msg_hash = Hash::new(msg);
    pubkey.verify(&msg_hash.as_bytes()[..], signature).is_ok()
}
