use p256::ecdsa::{signature::Signer, Signature, VerifyingKey};
use std::error::Error;

pub trait EcdsaKey: Signer<Signature> {
    type Error: Error + Send + Sync + 'static;

    fn verifying_key(&self) -> Result<VerifyingKey, Self::Error>;
}

pub trait EphemeralEcdsaKey: EcdsaKey {}

pub trait SecureEcdsaKey: EcdsaKey {}

impl EcdsaKey for p256::ecdsa::SigningKey {
    type Error = p256::ecdsa::Error;

    fn verifying_key(&self) -> Result<VerifyingKey, Self::Error> {
        Ok(*self.verifying_key())
    }
}
