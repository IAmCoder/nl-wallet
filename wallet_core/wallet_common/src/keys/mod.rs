use std::error::Error;

use p256::ecdsa::{Signature, VerifyingKey};

#[cfg(feature = "integration-test")]
pub mod integration_test;
#[cfg(feature = "software-keys")]
pub mod software;

pub trait EcdsaKey {
    type Error: Error + Send + Sync + 'static;

    async fn verifying_key(&self) -> Result<VerifyingKey, Self::Error>;

    /// Attempt to sign the given message, returning a digital signature on
    /// success, or an error if something went wrong.
    ///
    /// The main intended use case for signing errors is when communicating
    /// with external signers, e.g. cloud KMS, HSMs, or other hardware tokens.
    async fn try_sign(&self, msg: &[u8]) -> Result<Signature, Self::Error>;
}

/// Contract for ECDSA private keys which are short-lived and deterministically derived from a PIN.
pub trait EphemeralEcdsaKey: EcdsaKey {}

/// Contract for ECDSA private keys that are stored in some form of secure hardware from which they cannot be extracted,
/// e.g., a HSM, Android's TEE/StrongBox, or Apple's SE.
pub trait SecureEcdsaKey: EcdsaKey {}

// The `SigningKey` is an `EcdsaKey` but not a `SecureEcdsaKey` (except in mock/tests).
impl EcdsaKey for p256::ecdsa::SigningKey {
    type Error = p256::ecdsa::Error;

    async fn verifying_key(&self) -> Result<VerifyingKey, Self::Error> {
        Ok(*self.verifying_key())
    }

    async fn try_sign(&self, msg: &[u8]) -> Result<Signature, Self::Error> {
        p256::ecdsa::signature::Signer::try_sign(self, msg)
    }
}

/// The contract of this trait includes that a constructed type with the same
/// identifier behaves exactly the same, i.e. has the same key material backing it.
pub trait ConstructibleWithIdentifier: WithIdentifier {
    fn new(identifier: &str) -> Self
    where
        Self: Sized;
}

pub trait WithIdentifier {
    fn identifier(&self) -> &str;
}

/// Contract for encryption keys suitable for use in the wallet, e.g. for securely storing the database key.
/// Should be sufficiently secured e.g. through Android's TEE/StrongBox or Apple's SE.
/// Handles to private keys are requested through [`ConstructibleWithIdentifier::new()`].
pub trait SecureEncryptionKey: ConstructibleWithIdentifier {
    // from ConstructibleWithIdentifier: new(), identifier()
    type Error: Error + Send + Sync + 'static;

    async fn encrypt(&self, msg: &[u8]) -> Result<Vec<u8>, Self::Error>;
    async fn decrypt(&self, msg: &[u8]) -> Result<Vec<u8>, Self::Error>;
}

#[cfg(any(test, feature = "mock"))]
mod mock {
    impl super::EphemeralEcdsaKey for p256::ecdsa::SigningKey {}
    impl super::SecureEcdsaKey for p256::ecdsa::SigningKey {}
}
