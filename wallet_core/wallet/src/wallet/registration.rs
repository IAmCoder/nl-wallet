use std::error::Error;

use tracing::{info, instrument};

use platform_support::hw_keystore::PlatformEcdsaKey;
use wallet_common::{account::messages::auth::Registration, jwt::JwtError};

use crate::{
    account_provider::{AccountProviderClient, AccountProviderError},
    config::ConfigurationRepository,
    pin::{
        key::{self as pin_key, PinKey},
        validation::{validate_pin, PinValidationError},
    },
    storage::{RegistrationData, Storage, StorageError, StorageState},
};

use super::Wallet;

#[derive(Debug, thiserror::Error)]
pub enum WalletRegistrationError {
    #[error("wallet is already registered")]
    AlreadyRegistered,
    #[error("PIN provided for registration does not adhere to requirements: {0}")]
    InvalidPin(#[from] PinValidationError),
    #[error("could not request registration challenge from Wallet Provider: {0}")]
    ChallengeRequest(#[source] AccountProviderError),
    #[error("could not get hardware public key: {0}")]
    HardwarePublicKey(#[source] Box<dyn Error + Send + Sync>),
    #[error("could not sign registration message: {0}")]
    Signing(#[source] wallet_common::errors::Error),
    #[error("could not request registration from Wallet Provider: {0}")]
    RegistrationRequest(#[source] AccountProviderError),
    #[error("could not validate registration certificate received from Wallet Provider: {0}")]
    CertificateValidation(#[source] JwtError),
    #[error("public key in registration certificate received from Wallet Provider does not match hardware public key")]
    PublicKeyMismatch,
    #[error("could not store registration certificate in database: {0}")]
    StoreCertificate(#[from] StorageError),
}

impl<CR, S, PEK, APC, DGS, IS, MDS> Wallet<CR, S, PEK, APC, DGS, IS, MDS> {
    pub fn has_registration(&self) -> bool {
        self.registration.is_some()
    }

    #[instrument(skip_all)]
    pub async fn register(&mut self, pin: String) -> Result<(), WalletRegistrationError>
    where
        CR: ConfigurationRepository,
        S: Storage,
        APC: AccountProviderClient,
        PEK: PlatformEcdsaKey,
    {
        info!("Checking if already registered");

        // Registration is only allowed if we do not currently have a registration on record.
        if self.has_registration() {
            return Err(WalletRegistrationError::AlreadyRegistered);
        }

        info!("Validating PIN");

        // Make sure the PIN adheres to the requirements.
        validate_pin(&pin)?; // TODO: do not keep PIN in memory while request is in flight

        info!("Requesting challenge from account server");

        let config = &self.config_repository.config().account_server;
        let base_url = config.base_url.clone();
        let certificate_public_key = config.certificate_public_key.clone();

        // Retrieve a challenge from the account server
        let challenge = self
            .account_provider_client
            .registration_challenge(&base_url)
            .await
            .map_err(WalletRegistrationError::ChallengeRequest)?;

        info!("Challenge received from account server, signing and sending registration to account server");

        // Create a registration message and double sign it with the challenge.
        // Generate a new PIN salt and derive the private key from the provided PIN.
        let pin_salt = pin_key::new_pin_salt();
        let pin_key = PinKey::new(&pin, &pin_salt);

        // Retrieve the public key and sign the registration message.
        let hw_pubkey = self
            .hw_privkey
            .verifying_key()
            .await
            .map_err(|e| WalletRegistrationError::HardwarePublicKey(e.into()))?;
        let registration_message = Registration::new_signed(&self.hw_privkey, &pin_key, &challenge)
            .await
            .map_err(WalletRegistrationError::Signing)?;

        // Send the registration message to the account server and receive the wallet certificate in response.
        let wallet_certificate = self
            .account_provider_client
            .register(&base_url, registration_message)
            .await
            .map_err(WalletRegistrationError::RegistrationRequest)?;

        info!("Certificate received from account server, verifying contents");

        // Double check that the public key returned in the wallet certificate
        // matches that of our hardware key.
        let cert_claims = wallet_certificate
            .parse_and_verify_with_sub(&certificate_public_key.into())
            .map_err(WalletRegistrationError::CertificateValidation)?;
        if cert_claims.hw_pubkey.0 != hw_pubkey {
            return Err(WalletRegistrationError::PublicKeyMismatch);
        }

        info!("Storing received registration");

        // If the storage database does not exist, create it now.
        let storage = self.storage.get_mut();
        let storage_state = storage.state().await?;
        if !matches!(storage_state, StorageState::Opened) {
            storage.open().await?;
        }

        // Save the registration data in storage.
        let registration_data = RegistrationData {
            pin_salt,
            wallet_certificate,
        };
        storage.insert_data(&registration_data).await?;

        // Keep the registration data in memory.
        self.registration = Some(registration_data);

        // Unlock the wallet after successful registration
        self.lock.unlock();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;
    use http::StatusCode;
    use p256::ecdsa::SigningKey;
    use rand_core::OsRng;
    use wallet_common::{account::signed::SequenceNumberComparison, jwt::Jwt, utils};

    use crate::{account_provider::AccountProviderResponseError, wallet::test::ACCOUNT_SERVER_KEYS};

    use super::{super::test::WalletWithMocks, *};

    const PIN: &str = "051097";

    #[tokio::test]
    async fn test_wallet_register_success() {
        // Prepare an unregistered wallet.
        let mut wallet = WalletWithMocks::new_unregistered().await;

        // The wallet should report that it is currently unregistered and locked.
        assert!(!wallet.has_registration());
        assert!(wallet.storage.get_mut().data.is_empty());
        assert!(wallet.is_locked());

        // Have the account server respond with a random
        // challenge when the wallet sends a request for it.
        let challenge = utils::random_bytes(32);
        let challenge_response = challenge.clone();

        wallet
            .account_provider_client
            .expect_registration_challenge()
            .return_once(|_| Ok(challenge_response));

        // Have the account server respond with a valid
        // certificate when the wallet sends a request for it.
        let cert = wallet.valid_certificate().await;
        let cert_response = cert.clone();
        let challenge_expected = challenge.clone();

        wallet
            .account_provider_client
            .expect_register()
            .return_once(move |_, registration_signed| {
                let registration = registration_signed
                    .dangerous_parse_unverified()
                    .expect("Could not parse registration message");

                assert_eq!(registration.challenge, challenge_expected);

                registration_signed
                    .parse_and_verify(
                        &registration.challenge,
                        SequenceNumberComparison::EqualTo(0),
                        &registration.payload.hw_pubkey.0,
                        &registration.payload.pin_pubkey.0,
                    )
                    .expect("Could not verify registration message");

                Ok(cert_response)
            });

        // Register the wallet with a valid PIN.
        wallet
            .register(PIN.to_string())
            .await
            .expect("Could not register wallet");

        // The wallet should now report that it is registered and unlocked.
        assert!(wallet.has_registration());
        assert!(!wallet.is_locked());

        // The registration should be stored in the database.
        let stored_registration: RegistrationData = wallet
            .storage
            .get_mut()
            .fetch_data()
            .await
            .unwrap()
            .expect("Registration data not present in storage");
        assert_eq!(stored_registration.wallet_certificate.0, cert.0);
    }

    #[tokio::test]
    async fn test_wallet_register_error_already_registered() {
        let mut wallet = WalletWithMocks::new_registered_and_unlocked().await;

        let error = wallet
            .register(PIN.to_string())
            .await
            .expect_err("Wallet registration should have resulted in error");

        assert_matches!(error, WalletRegistrationError::AlreadyRegistered);
        assert!(wallet.has_registration());
    }

    #[tokio::test]
    async fn test_wallet_register_error_invalid_pin() {
        let mut wallet = WalletWithMocks::new_unregistered().await;

        // Try to register with an insecure PIN.
        let error = wallet
            .register("123456".to_string())
            .await
            .expect_err("Wallet registration should have resulted in error");

        assert_matches!(error, WalletRegistrationError::InvalidPin(_));
        assert!(!wallet.has_registration());
        assert!(wallet.storage.get_mut().data.is_empty());
    }

    #[tokio::test]
    async fn test_wallet_register_error_challenge_request() {
        let mut wallet = WalletWithMocks::new_unregistered().await;

        // Have the account server respond to the challenge request with a 500 error.
        wallet
            .account_provider_client
            .expect_registration_challenge()
            .return_once(|_| Err(AccountProviderResponseError::Status(StatusCode::INTERNAL_SERVER_ERROR).into()));

        let error = wallet
            .register(PIN.to_string())
            .await
            .expect_err("Wallet registration should have resulted in error");

        assert_matches!(error, WalletRegistrationError::ChallengeRequest(_));
        assert!(!wallet.has_registration());
        assert!(wallet.storage.get_mut().data.is_empty());
    }

    #[tokio::test]
    async fn test_wallet_register_error_hardware_public_key() {
        let mut wallet = WalletWithMocks::new_unregistered().await;

        wallet
            .account_provider_client
            .expect_registration_challenge()
            .return_once(|_| Ok(utils::random_bytes(32)));

        // Have the hardware public key fetching fail.
        wallet
            .hw_privkey
            .next_public_key_error
            .lock()
            .unwrap()
            .replace(p256::ecdsa::Error::new());

        let error = wallet
            .register(PIN.to_string())
            .await
            .expect_err("Wallet registration should have resulted in error");

        assert_matches!(error, WalletRegistrationError::HardwarePublicKey(_));
        assert!(!wallet.has_registration());
        assert!(wallet.storage.get_mut().data.is_empty());
    }

    #[tokio::test]
    async fn test_wallet_register_error_signing() {
        let mut wallet = WalletWithMocks::new_unregistered().await;

        wallet
            .account_provider_client
            .expect_registration_challenge()
            .return_once(|_| Ok(utils::random_bytes(32)));

        // Have the hardware key signing fail.
        wallet
            .hw_privkey
            .next_private_key_error
            .lock()
            .unwrap()
            .replace(p256::ecdsa::Error::new());

        let error = wallet
            .register(PIN.to_string())
            .await
            .expect_err("Wallet registration should have resulted in error");

        assert_matches!(error, WalletRegistrationError::Signing(_));
        assert!(!wallet.has_registration());
        assert!(wallet.storage.get_mut().data.is_empty());
    }

    #[tokio::test]
    async fn test_wallet_register_error_registration_request() {
        let mut wallet = WalletWithMocks::new_unregistered().await;

        wallet
            .account_provider_client
            .expect_registration_challenge()
            .return_once(|_| Ok(utils::random_bytes(32)));

        // Have the account server respond to the registration request with a 401 error.
        wallet
            .account_provider_client
            .expect_register()
            .return_once(|_, _| Err(AccountProviderResponseError::Status(StatusCode::UNAUTHORIZED).into()));

        let error = wallet
            .register(PIN.to_string())
            .await
            .expect_err("Wallet registration should have resulted in error");

        assert_matches!(error, WalletRegistrationError::RegistrationRequest(_));
        assert!(!wallet.has_registration());
        assert!(wallet.storage.get_mut().data.is_empty());
    }

    #[tokio::test]
    async fn test_wallet_register_error_certificate_validation() {
        let mut wallet = WalletWithMocks::new_unregistered().await;

        wallet
            .account_provider_client
            .expect_registration_challenge()
            .return_once(|_| Ok(utils::random_bytes(32)));

        // Have the account server sign the wallet certificate with
        // a key to which the certificate public key does not belong.
        let other_key = SigningKey::random(&mut OsRng);
        let cert = Jwt::sign_with_sub(&wallet.valid_certificate_claims().await, &other_key)
            .await
            .unwrap();

        wallet
            .account_provider_client
            .expect_register()
            .return_once(|_, _| Ok(cert));

        let error = wallet
            .register(PIN.to_string())
            .await
            .expect_err("Wallet registration should have resulted in error");

        assert_matches!(error, WalletRegistrationError::CertificateValidation(_));
        assert!(!wallet.has_registration());
        assert!(wallet.storage.get_mut().data.is_empty());
    }

    #[tokio::test]
    async fn test_wallet_register_error_public_key_mismatch() {
        let mut wallet = WalletWithMocks::new_unregistered().await;

        wallet
            .account_provider_client
            .expect_registration_challenge()
            .return_once(|_| Ok(utils::random_bytes(32)));

        // Have the account server include a hardware public key
        // in the wallet certificate that the wallet did not send.
        let other_key = SigningKey::random(&mut OsRng);
        let mut cert_claims = wallet.valid_certificate_claims().await;
        cert_claims.hw_pubkey = (*other_key.verifying_key()).into();
        let cert = Jwt::sign_with_sub(&cert_claims, &ACCOUNT_SERVER_KEYS.certificate_signing_key)
            .await
            .unwrap();

        wallet
            .account_provider_client
            .expect_register()
            .return_once(|_, _| Ok(cert));

        let error = wallet
            .register(PIN.to_string())
            .await
            .expect_err("Wallet registration should have resulted in error");

        assert_matches!(error, WalletRegistrationError::PublicKeyMismatch);
        assert!(!wallet.has_registration());
        assert!(wallet.storage.get_mut().data.is_empty());
    }

    #[tokio::test]
    async fn test_wallet_register_error_store_certificate() {
        let mut wallet = WalletWithMocks::new_unregistered().await;

        wallet
            .account_provider_client
            .expect_registration_challenge()
            .return_once(|_| Ok(utils::random_bytes(32)));

        let cert = wallet.valid_certificate().await;

        wallet
            .account_provider_client
            .expect_register()
            .return_once(|_, _| Ok(cert));

        // Have the database return an error
        // when inserting the wallet certificate.
        wallet.storage.get_mut().has_query_error = true;

        let error = wallet
            .register(PIN.to_string())
            .await
            .expect_err("Wallet registration should have resulted in error");

        assert_matches!(error, WalletRegistrationError::StoreCertificate(_));
        assert!(!wallet.has_registration());
        assert!(wallet.storage.get_mut().data.is_empty());
    }
}
