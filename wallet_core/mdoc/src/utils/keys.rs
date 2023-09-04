use wallet_common::keys::{ConstructibleWithIdentifier, SecureEcdsaKey};

/// Contract for ECDSA private keys suitable for mdoc attestations.
/// Should be sufficiently secured e.g. through a HSM, or Android's TEE/StrongBox or Apple's SE.
/// Handles to private keys are requested through [`ConstructibleWithIdentifier::new()`].
pub trait MdocEcdsaKey: ConstructibleWithIdentifier + SecureEcdsaKey {
    const KEY_TYPE: &'static str;

    // from ConstructibleWithIdentifier: new(), identifier()
    // from SecureSigningKey: verifying_key(), try_sign() and sign() methods
}

#[cfg(any(test, feature = "mock"))]
mod mock {
    use wallet_common::keys::software::SoftwareEcdsaKey;

    use super::MdocEcdsaKey;

    impl MdocEcdsaKey for SoftwareEcdsaKey {
        const KEY_TYPE: &'static str = "software";
    }
}
