use p256::ecdsa::VerifyingKey;

use crate::{errors::Result, iso::disclosure::IssuerSigned};

impl IssuerSigned {
    pub fn public_key(&self) -> Result<VerifyingKey> {
        let public_key = self
            .issuer_auth
            .dangerous_parse_unverified()?
            .0
            .device_key_info
            .try_into()?;
        Ok(public_key)
    }
}

#[cfg(test)]
mod tests {
    use wallet_common::keys::{software::SoftwareEcdsaKey, ConstructibleWithIdentifier, EcdsaKey};

    use crate::holder::Mdoc;

    #[tokio::test]
    async fn test_issuer_signed_public_key() {
        let mdoc = Mdoc::new_example_mock();

        let public_key = mdoc
            .issuer_signed
            .public_key()
            .expect("Could not get public key from from IssuerSigned");

        let expected_public_key = SoftwareEcdsaKey::new(&mdoc.private_key_id)
            .verifying_key()
            .await
            .unwrap();

        assert_eq!(public_key, expected_public_key);
    }
}
