use std::error::Error;

use tracing::info;
use tracing::instrument;

use error_category::sentry_capture_error;
use error_category::ErrorCategory;
use platform_support::attested_key::AttestedKey;
use platform_support::attested_key::AttestedKeyHolder;
use platform_support::attested_key::KeyWithAttestation;
use wallet_common::account::messages::auth::Registration;
use wallet_common::account::signed::ChallengeResponse;
use wallet_common::jwt::JwtError;
use wallet_common::keys::EcdsaKey;
use wallet_common::utils;

use crate::account_provider::AccountProviderClient;
use crate::account_provider::AccountProviderError;
use crate::config::ConfigurationRepository;
use crate::pin::key::PinKey;
use crate::pin::key::{self as pin_key};
use crate::pin::validation::validate_pin;
use crate::pin::validation::PinValidationError;
use crate::storage::RegistrationData;
use crate::storage::Storage;
use crate::storage::StorageError;
use crate::storage::StorageState;

use super::Wallet;
use super::WalletRegistration;

#[derive(Debug, thiserror::Error, ErrorCategory)]
#[category(defer)]
pub enum WalletRegistrationError {
    #[error("wallet is already registered")]
    #[category(expected)]
    AlreadyRegistered,
    #[error("PIN provided for registration does not adhere to requirements: {0}")]
    InvalidPin(#[from] PinValidationError),
    #[error("could not request registration challenge from Wallet Provider: {0}")]
    ChallengeRequest(#[source] AccountProviderError),
    #[error("could not generate attested key: {0}")]
    #[category(pd)]
    KeyGeneration(#[source] Box<dyn Error + Send + Sync>),
    #[error("could not perform key and/or app attestation: {0}")]
    #[category(pd)]
    Attestation(#[source] Box<dyn Error + Send + Sync>),
    #[category(pd)]
    #[error("could not get attested public key: {0}")]
    AttestedPublicKey(#[source] Box<dyn Error + Send + Sync>),
    #[error("could not sign registration message: {0}")]
    Signing(#[source] wallet_common::account::errors::Error),
    #[error("could not request registration from Wallet Provider: {0}")]
    RegistrationRequest(#[source] AccountProviderError),
    #[error("could not validate registration certificate received from Wallet Provider: {0}")]
    CertificateValidation(#[source] JwtError),
    #[error("public key in registration certificate received from Wallet Provider does not match hardware public key")]
    #[category(expected)] // This error can happen during development, but should not happen in production
    PublicKeyMismatch,
    #[error("could not store registration certificate in database: {0}")]
    StoreCertificate(#[from] StorageError),
}

impl<CR, S, AKH, APC, DS, IS, MDS, WIC> Wallet<CR, S, AKH, APC, DS, IS, MDS, WIC>
where
    AKH: AttestedKeyHolder,
{
    pub fn has_registration(&self) -> bool {
        self.registration.is_registered()
    }

    #[instrument(skip_all)]
    #[sentry_capture_error]
    pub async fn register(&mut self, pin: String) -> Result<(), WalletRegistrationError>
    where
        CR: ConfigurationRepository,
        S: Storage,
        APC: AccountProviderClient,
    {
        info!("Checking if already registered");

        // Registration is only allowed if we do not currently have a registration on record.
        if self.has_registration() {
            return Err(WalletRegistrationError::AlreadyRegistered);
        }

        info!("Validating PIN");

        // Make sure the PIN adheres to the requirements.
        validate_pin(&pin)?; // TODO: do not keep PIN in memory while request is in flight (PVW-1290)

        info!("Requesting challenge from account server");

        let config = &self.config_repository.config().account_server;
        let certificate_public_key = config.certificate_public_key.clone();

        // Retrieve a challenge from the account server
        let challenge = self
            .account_provider_client
            .registration_challenge(&config.http_config)
            .await
            .map_err(WalletRegistrationError::ChallengeRequest)?;

        info!("Challenge received from account server, generating attested key");

        let key_identifier = self
            .key_holder
            .generate()
            .await
            .map_err(|error| WalletRegistrationError::KeyGeneration(Box::new(error)))?;

        info!("Performing key and app attestation");

        // TODO: Save key identifier when error is retryable and on success.

        let key_with_attestation = self
            .key_holder
            .attest(key_identifier.clone(), utils::sha256(&challenge))
            .await
            .map_err(|error| WalletRegistrationError::Attestation(Box::new(error.error)))?;

        info!("Key and app attestation successful, signing and sending registration to account server");

        // Create a registration message and double sign it with the challenge.
        // Generate a new PIN salt and derive the private key from the provided PIN.
        let pin_salt = pin_key::new_pin_salt();
        let pin_key = PinKey::new(&pin, &pin_salt);

        // Sign the registration message based on the attestation type.
        let (registration_message, attested_key) = match key_with_attestation {
            KeyWithAttestation::Apple { key, attestation_data } => {
                ChallengeResponse::<Registration>::new_apple(&key, attestation_data, &pin_key, challenge)
                    .await
                    .map(|message| (message, AttestedKey::Apple(key)))
            }
            // TODO: Support Google attestation.
            KeyWithAttestation::Google { key, .. } => {
                ChallengeResponse::<Registration>::new_unattested(&key, &pin_key, challenge)
                    .await
                    .map(|message| (message, AttestedKey::Google(key)))
            }
        }
        .map_err(WalletRegistrationError::Signing)?;

        // Send the registration message to the account server and receive the wallet certificate in response.
        let wallet_certificate = self
            .account_provider_client
            .register(&config.http_config, registration_message)
            .await
            .map_err(WalletRegistrationError::RegistrationRequest)?;

        info!("Certificate received from account server, verifying contents");

        // Double check that the public key returned in the wallet certificate matches that of our hardware key.
        // Note that this public key is only available on Android, on iOS all we have is opaque attestation data.
        let cert_claims = wallet_certificate
            .parse_and_verify_with_sub(&certificate_public_key.into())
            .map_err(WalletRegistrationError::CertificateValidation)?;

        if let AttestedKey::Google(key) = &attested_key {
            let attested_pub_key = key
                .verifying_key()
                .await
                .map_err(|error| WalletRegistrationError::AttestedPublicKey(Box::new(error)))?;

            if cert_claims.hw_pubkey.0 != attested_pub_key {
                return Err(WalletRegistrationError::PublicKeyMismatch);
            }
        }

        info!("Storing received registration");

        // If the storage database does not exist, create it now.
        let storage = self.storage.get_mut();
        let storage_state = storage.state().await?;
        if !matches!(storage_state, StorageState::Opened) {
            storage.open().await?;
        }

        // Save the registration data in storage.
        let data = RegistrationData {
            attested_key_identifier: key_identifier,
            wallet_id: cert_claims.wallet_id,
            pin_salt,
            wallet_certificate,
        };
        storage.insert_data(&data).await?;

        // Keep the registration data in memory.
        self.registration = WalletRegistration::Registered { attested_key, data };

        // Unlock the wallet after successful registration
        self.lock.unlock();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use assert_matches::assert_matches;
    use futures::FutureExt;
    use http::StatusCode;
    use p256::ecdsa::SigningKey;
    use parking_lot::Mutex;
    use rand_core::OsRng;

    use apple_app_attest::AssertionCounter;
    use apple_app_attest::AttestationEnvironment;
    use apple_app_attest::VerifiedAttestation;
    use platform_support::attested_key::mock::KeyHolderErrorScenario;
    use wallet_common::account::messages::auth::RegistrationAttestation;
    use wallet_common::account::messages::auth::WalletCertificate;
    use wallet_common::account::signed::SequenceNumberComparison;
    use wallet_common::jwt::Jwt;
    use wallet_common::utils;

    use crate::account_provider::AccountProviderResponseError;
    use crate::storage::KeyedData;
    use crate::storage::KeyedDataResult;

    use super::super::test::WalletWithMocks;
    use super::*;

    const PIN: &str = "051097";

    // TODO: Add test for registration using Google attested key.

    #[tokio::test]
    async fn test_wallet_register_success_apple() {
        // Prepare an unregistered wallet.
        let mut wallet = WalletWithMocks::new_unregistered();

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
        // let cert = wallet.valid_certificate().await;
        // let cert_response = cert.clone();
        let trust_anchor = wallet.key_holder.ca.trust_anchor().to_owned();
        let app_identifier = wallet.key_holder.app_identifier.clone();
        let challenge_expected = challenge.clone();

        // Set up a mutex for the mock callback to write the generated wallet certificate to.
        let generated_certificate: Arc<Mutex<Option<WalletCertificate>>> = Arc::new(Mutex::new(None));
        let generated_certificate_clone = Arc::clone(&generated_certificate);

        wallet
            .account_provider_client
            .expect_register()
            .return_once(move |_, registration_signed| {
                let registration = registration_signed
                    .dangerous_parse_unverified()
                    .expect("registration message should parse");

                assert_eq!(registration.challenge, challenge_expected);

                let RegistrationAttestation::Apple { data: attestation_data } = &registration.payload.attestation
                else {
                    panic!("registration message should contain Apple attestation");
                };

                // Verify the mock attestaiton in order to get the public key.
                let (_, attested_public_key) = VerifiedAttestation::parse_and_verify(
                    attestation_data,
                    &[trust_anchor],
                    &utils::sha256(&registration.challenge),
                    &app_identifier,
                    AttestationEnvironment::Development,
                )
                .expect("registration message Apple attestation should verify");

                // Verify the registration message, both counters start at 0.
                registration_signed
                    .parse_and_verify_apple(
                        &registration.challenge,
                        SequenceNumberComparison::EqualTo(0),
                        &attested_public_key,
                        &app_identifier,
                        AssertionCounter::default(),
                        &registration.payload.pin_pubkey.0,
                    )
                    .expect("registration message should verify");

                // Generate a valid certificate and wallet id based on on the public key.
                let certificate = WalletWithMocks::valid_certificate(None, attested_public_key);
                generated_certificate_clone.lock().replace(certificate.clone());

                Ok(certificate)
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
        assert_eq!(
            stored_registration.wallet_certificate.0,
            generated_certificate.lock().as_ref().unwrap().0
        );
    }

    #[tokio::test]
    async fn test_wallet_register_error_already_registered() {
        let mut wallet = WalletWithMocks::new_registered_and_unlocked_apple();

        let error = wallet
            .register(PIN.to_string())
            .await
            .expect_err("Wallet registration should have resulted in error");

        assert_matches!(error, WalletRegistrationError::AlreadyRegistered);
        assert!(wallet.has_registration());
    }

    #[tokio::test]
    async fn test_wallet_register_error_invalid_pin() {
        let mut wallet = WalletWithMocks::new_unregistered();

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
        let mut wallet = WalletWithMocks::new_unregistered();

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
    async fn test_wallet_register_error_attestation() {
        let mut wallet = WalletWithMocks::new_unregistered();

        wallet
            .account_provider_client
            .expect_registration_challenge()
            .return_once(|_| Ok(utils::random_bytes(32)));

        // Have the hardware key signing fail.
        wallet.key_holder.error_scenario = KeyHolderErrorScenario::UnretryableAttestationError;

        let error = wallet
            .register(PIN.to_string())
            .await
            .expect_err("Wallet registration should have resulted in error");

        assert_matches!(error, WalletRegistrationError::Attestation(_));
        assert!(!wallet.has_registration());
        assert!(wallet.storage.get_mut().data.is_empty());
    }

    #[tokio::test]
    async fn test_wallet_register_error_registration_request() {
        let mut wallet = WalletWithMocks::new_unregistered();

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
        let mut wallet = WalletWithMocks::new_unregistered();

        wallet
            .account_provider_client
            .expect_registration_challenge()
            .return_once(|_| Ok(utils::random_bytes(32)));

        // Have the account server sign the wallet certificate with
        // a key to which the certificate public key does not belong.
        wallet.account_provider_client.expect_register().return_once(|_, _| {
            let other_account_server_key = SigningKey::random(&mut OsRng);
            // Note that this key does not get checked by the wallet for Apple attestation.
            let random_pubkey = *SigningKey::random(&mut OsRng).verifying_key();

            let certificate = Jwt::sign_with_sub(
                &WalletWithMocks::valid_certificate_claims(None, random_pubkey),
                &other_account_server_key,
            )
            .now_or_never()
            .unwrap()
            .unwrap();

            Ok(certificate)
        });

        let error = wallet
            .register(PIN.to_string())
            .await
            .expect_err("Wallet registration should have resulted in error");

        assert_matches!(error, WalletRegistrationError::CertificateValidation(_));
        assert!(!wallet.has_registration());
        assert!(wallet.storage.get_mut().data.is_empty());
    }

    // TODO: Test WalletRegistrationError::PublicKeyMismatch error with Google attestation.

    #[tokio::test]
    async fn test_wallet_register_error_store_certificate() {
        let mut wallet = WalletWithMocks::new_unregistered();

        wallet
            .account_provider_client
            .expect_registration_challenge()
            .return_once(|_| Ok(utils::random_bytes(32)));

        wallet.account_provider_client.expect_register().return_once(|_, _| {
            // Note that this key does not get checked by the wallet for Apple attestation.
            let random_pubkey = *SigningKey::random(&mut OsRng).verifying_key();
            let certificate = WalletWithMocks::valid_certificate(None, random_pubkey);

            Ok(certificate)
        });

        // Have the database return an error
        // when inserting the wallet certificate.
        wallet.storage.get_mut().set_keyed_data_error(RegistrationData::KEY);

        let error = wallet
            .register(PIN.to_string())
            .await
            .expect_err("Wallet registration should have resulted in error");

        assert_matches!(error, WalletRegistrationError::StoreCertificate(_));
        assert!(!wallet.has_registration());
        assert_matches!(
            wallet.storage.get_mut().data.get(RegistrationData::KEY),
            Some(KeyedDataResult::Error)
        );
    }
}
