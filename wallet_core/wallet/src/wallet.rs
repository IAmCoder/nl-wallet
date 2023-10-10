use std::error::Error;

use futures::future::TryFutureExt;
use p256::ecdsa::signature;
use tokio::sync::RwLock;
use tracing::{info, instrument, warn};
use url::Url;

use platform_support::{
    hw_keystore::{
        hardware::{HardwareEcdsaKey, HardwareEncryptionKey},
        PlatformEcdsaKey,
    },
    utils::hardware::HardwareUtilities,
};
use wallet_common::account::messages::{auth::Registration, instructions::CheckPin};

use crate::{
    account_provider::{AccountProviderClient, AccountProviderError, HttpAccountProviderClient},
    config::{Configuration, ConfigurationRepository, LocalConfigurationRepository},
    digid::{DigidError, DigidSession, HttpDigidSession},
    document::{Document, DocumentMdocError, DocumentPersistence},
    instruction::{InstructionClient, InstructionError, RemoteEcdsaKeyError, RemoteEcdsaKeyFactory},
    lock::WalletLock,
    pid_issuer::{HttpPidIssuerClient, PidIssuerClient, PidIssuerError},
    pin::{
        key::{new_pin_salt, PinKey},
        validation::{validate_pin, PinValidationError},
    },
    storage::{DatabaseStorage, RegistrationData, Storage, StorageError, StorageState},
};

const WALLET_KEY_ID: &str = "wallet";

#[derive(Debug, thiserror::Error)]
pub enum WalletInitError {
    #[error("could not initialize database: {0}")]
    Database(#[from] StorageError),
}

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
    CertificateValidation(#[source] wallet_common::errors::Error),
    #[error("public key in registration certificate received from Wallet Provider does not match hardware public key")]
    PublicKeyMismatch,
    #[error("could not store registration certificate in database: {0}")]
    StoreCertificate(#[from] StorageError),
}

#[derive(Debug, thiserror::Error)]
pub enum WalletUnlockError {
    #[error("wallet is not registered")]
    NotRegistered,
    #[error("could not retrieve registration from database: {0}")]
    Database(#[from] StorageError),
    #[error("could not get hardware public key: {0}")]
    HardwarePublicKey(#[source] Box<dyn Error + Send + Sync>),
    #[error("error sending instruction to Wallet Provider: {0}")]
    Instruction(#[from] InstructionError),
}

#[derive(Debug, thiserror::Error)]
pub enum PidIssuanceError {
    #[error("wallet is not registered")]
    NotRegistered,
    #[error("could not start DigiD session: {0}")]
    DigidSessionStart(#[source] DigidError),
    #[error("no DigiD session was found")]
    NoSession,
    #[error("could not finish DigiD session: {0}")]
    DigidSessionFinish(#[source] DigidError),
    #[error("could not retrieve PID from issuer: {0}")]
    PidIssuer(#[source] PidIssuerError),
    #[error("error sending instruction to Wallet Provider: {0}")]
    Instruction(#[from] InstructionError),
    #[error("invalid signature received from Wallet Provider: {0}")]
    Signature(#[from] signature::Error),
    #[error("could not interpret mdoc attributes: {0}")]
    Document(#[from] DocumentMdocError),
    #[error("could not access mdocs database: {0}")]
    Database(#[from] StorageError),
    #[error("key '{0}' not found in Wallet Provider")]
    KeyNotFound(String),
}

#[derive(Debug, thiserror::Error)]
pub enum SetDocumentsCallbackError {
    #[error("Could not fetch mdocs from database storage: {0}")]
    Storage(#[from] StorageError),
}

pub enum RedirectUriType {
    PidIssuance,
    Unknown,
}

type ConfigurationCallback = Box<dyn FnMut(&Configuration) + Send + Sync>;
type DocumentsCallback = Box<dyn FnMut(Vec<Document>) + Send + Sync>;

pub struct Wallet<
    C = LocalConfigurationRepository,
    S = DatabaseStorage<HardwareEncryptionKey>,
    K = HardwareEcdsaKey,
    A = HttpAccountProviderClient,
    D = HttpDigidSession,
    P = HttpPidIssuerClient,
> {
    config_repository: C,
    storage: RwLock<S>,
    hw_privkey: K,
    account_provider_client: A,
    digid_session: Option<D>,
    pid_issuer: P,
    lock: WalletLock,
    registration: Option<RegistrationData>,
    config_callback: Option<ConfigurationCallback>,
    documents_callback: Option<DocumentsCallback>,
}

impl Wallet {
    pub async fn init_all() -> Result<Self, WalletInitError> {
        #[cfg(feature = "disable_tls_validation")]
        tracing::warn!("TLS validation disabled");

        let storage = DatabaseStorage::<HardwareEncryptionKey>::init::<HardwareUtilities>().await?;

        Self::init_registration(
            LocalConfigurationRepository::default(),
            storage,
            HttpAccountProviderClient::default(),
            HttpPidIssuerClient::default(),
        )
        .await
    }
}

impl<C, S, K, A, D, P> Wallet<C, S, K, A, D, P>
where
    C: ConfigurationRepository,
    S: Storage + Send + Sync,
    K: PlatformEcdsaKey + Sync,
    A: AccountProviderClient + Sync,
    D: DigidSession,
    P: PidIssuerClient,
{
    /// Initialize the wallet by loading initial state.
    pub async fn init_registration(
        config_repository: C,
        mut storage: S,
        account_provider_client: A,
        pid_issuer: P,
    ) -> Result<Self, WalletInitError> {
        let registration = Self::fetch_registration(&mut storage).await?;

        let wallet = Wallet {
            config_repository,
            storage: RwLock::new(storage),
            hw_privkey: K::new(WALLET_KEY_ID),
            account_provider_client,
            digid_session: None,
            pid_issuer,
            lock: WalletLock::new(true),
            registration,
            config_callback: None,
            documents_callback: None,
        };

        Ok(wallet)
    }

    /// Attempts to fetch the registration data from storage, without creating a database if there is none.
    async fn fetch_registration(storage: &mut S) -> Result<Option<RegistrationData>, StorageError> {
        match storage.state().await? {
            // If there is no database file, we can conclude early that there is no registration.
            StorageState::Uninitialized => return Ok(None),
            // Open the database, if necessary.
            StorageState::Unopened => storage.open().await?,
            StorageState::Opened => (),
        }

        // Finally, fetch the registration.
        storage.fetch_data::<RegistrationData>().await
    }

    pub fn set_lock_callback<F>(&mut self, mut callback: F)
    where
        F: FnMut(bool) + Send + Sync + 'static,
    {
        callback(self.lock.is_locked());
        self.lock.set_lock_callback(callback);
    }

    pub fn clear_lock_callback(&mut self) {
        self.lock.clear_lock_callback()
    }

    pub fn set_config_callback<F>(&mut self, mut callback: F)
    where
        F: FnMut(&Configuration) + Send + Sync + 'static,
    {
        callback(self.config_repository.config());
        // TODO: Once configuration fetching from the Wallet Provider is implemented,
        //       this callback should be called every time the config updates.
        self.config_callback.replace(Box::new(callback));
    }

    pub fn clear_config_callback(&mut self) {
        self.config_callback.take();
    }

    pub async fn set_documents_callback<F>(&mut self, callback: F) -> Result<(), SetDocumentsCallbackError>
    where
        F: FnMut(Vec<Document>) + Send + Sync + 'static,
    {
        self.documents_callback.replace(Box::new(callback));

        // If the `Wallet` is not registered, the database will not be open.
        // In that case send an empty vec, so the UI has something to work with.
        //
        // TODO: have the UI not call this until after registration.
        if self.has_registration() {
            self.emit_documents().await?;
        } else {
            self.documents_callback.as_mut().unwrap()(Default::default());
        }

        Ok(())
    }

    async fn emit_documents(&mut self) -> Result<(), StorageError> {
        info!("Emit mdocs from storage");

        let storage = self.storage.read().await;

        // Note that this currently panics whenever conversion from Mdoc to Documents fails,
        // as we assume that the (hardcoded) mapping will always be backwards compatible.
        // This is particularly important when this mapping comes from a trusted registry
        // in the near future!
        let mut documents = storage
            .fetch_unique_mdocs()
            .await?
            .into_iter()
            .map(|(id, mdoc)| {
                Document::from_mdoc_attributes(
                    DocumentPersistence::Stored(id.to_string()),
                    &mdoc.doc_type,
                    mdoc.attributes(),
                )
                .expect("Could not interpret stored mdoc attributes")
            })
            .collect::<Vec<_>>();

        documents.sort_by_key(Document::priority);

        if let Some(ref mut callback) = self.documents_callback {
            callback(documents);
        }

        Ok(())
    }

    pub fn clear_documents_callback(&mut self) {
        self.documents_callback.take();
    }

    pub fn has_registration(&self) -> bool {
        self.registration.is_some()
    }

    pub fn is_locked(&self) -> bool {
        self.lock.is_locked()
    }

    pub fn lock(&mut self) {
        self.lock.lock()
    }

    #[instrument(skip_all)]
    pub async fn register(&mut self, pin: String) -> Result<(), WalletRegistrationError> {
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
        let pin_salt = new_pin_salt();
        let pin_key = PinKey::new(&pin, &pin_salt);

        // Retrieve the public key and sign the registration message (these calls may block).
        let hw_pubkey = self
            .hw_privkey
            .verifying_key()
            .await
            .map_err(|e| WalletRegistrationError::HardwarePublicKey(e.into()))?;
        let registration_message = Registration::new_signed(&self.hw_privkey, &pin_key, &challenge)
            .await
            .map_err(WalletRegistrationError::Signing)?;

        // Send the registration message to the account server and receive the wallet certificate in response.
        let cert = self
            .account_provider_client
            .register(&base_url, registration_message)
            .await
            .map_err(WalletRegistrationError::RegistrationRequest)?;

        info!("Certificate received from account server, verifying contents");

        // Double check that the public key returned in the wallet certificate
        // matches that of our hardware key.
        let cert_claims = cert
            .parse_and_verify(&certificate_public_key)
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
            pin_salt: pin_salt.into(),
            wallet_certificate: cert,
        };
        storage.insert_data(&registration_data).await?;

        // Keep the registration data in memory.
        self.registration = Some(registration_data);

        // Unlock the wallet after successful registration
        self.lock.unlock();

        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn unlock(&mut self, pin: String) -> Result<(), WalletUnlockError> {
        info!("Validating pin");

        info!("Checking if already registered");
        if !self.has_registration() {
            return Err(WalletUnlockError::NotRegistered);
        }

        let registration_data = self.registration.as_ref().unwrap();
        let config = self.config_repository.config();

        let remote_instruction = InstructionClient::new(
            pin,
            &self.storage,
            &self.hw_privkey,
            &self.account_provider_client,
            registration_data,
            &config.account_server.base_url,
            &config.account_server.instruction_result_public_key,
        );

        remote_instruction
            .send(CheckPin)
            .inspect_ok(|_| self.lock.unlock())
            .await?;

        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn create_pid_issuance_auth_url(&mut self) -> Result<Url, PidIssuanceError> {
        info!("Generating DigiD auth URL, starting OpenID connect discovery");

        info!("Checking if already registered");
        if !self.has_registration() {
            return Err(PidIssuanceError::NotRegistered);
        }

        if self.digid_session.is_some() {
            warn!("DigiD auth url is requested for PID issuance while another Digid session is present, overwriting");
        }

        let pid_issuance_config = &self.config_repository.config().pid_issuance;

        let session = D::start(
            pid_issuance_config.digid_url.clone(),
            pid_issuance_config.digid_client_id.to_string(),
            pid_issuance_config.digid_redirect_uri.clone(),
        )
        .await
        .map_err(PidIssuanceError::DigidSessionStart)?;

        info!("DigiD auth URL generated");

        let auth_url = session.auth_url();
        self.digid_session.replace(session);

        Ok(auth_url)
    }

    pub fn identify_redirect_uri(&self, redirect_uri: &Url) -> RedirectUriType {
        if self
            .digid_session
            .as_ref()
            .map(|session| session.matches_received_redirect_uri(redirect_uri))
            .unwrap_or_default()
        {
            return RedirectUriType::PidIssuance;
        }

        RedirectUriType::Unknown
    }

    pub fn cancel_pid_issuance(&mut self) {
        if self.digid_session.is_none() {
            warn!("PID issuance was cancelled, but no DigiD session is currently present");

            return;
        }

        info!("PID issuance cancelled, removing DigiD session");

        self.digid_session.take();
    }

    #[instrument(skip_all)]
    pub async fn continue_pid_issuance(&mut self, redirect_uri: &Url) -> Result<Vec<Document>, PidIssuanceError> {
        info!("Received DigiD redirect URI, processing URI and retrieving access token");

        info!("Checking if already registered");
        if !self.has_registration() {
            return Err(PidIssuanceError::NotRegistered);
        }

        // Try to take ownership of any active `DigidSession`.
        let session = self.digid_session.take().ok_or(PidIssuanceError::NoSession)?;

        let access_token = session
            .get_access_token(redirect_uri)
            .await
            .map_err(PidIssuanceError::DigidSessionFinish)?;

        info!("DigiD access token retrieved, starting actual PID issuance");

        let config = self.config_repository.config();

        let unsigned_mdocs = self
            .pid_issuer
            .start_retrieve_pid(&config.pid_issuance.pid_issuer_url, &access_token)
            .await
            .map_err(PidIssuanceError::PidIssuer)?;

        info!("PID received successfully from issuer");

        let mut documents = unsigned_mdocs
            .into_iter()
            .map(Document::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        documents.sort_by_key(Document::priority);

        Ok(documents)
    }

    #[instrument(skip_all)]
    pub async fn accept_pid_issuance(&mut self, pin: String) -> Result<(), PidIssuanceError> {
        info!("Accepting PID issuance");

        info!("Checking if already registered");
        if !self.has_registration() {
            return Err(PidIssuanceError::NotRegistered);
        }

        let registration_data = self.registration.as_ref().unwrap();
        let config = self.config_repository.config();

        let remote_instruction = InstructionClient::new(
            pin,
            &self.storage,
            &self.hw_privkey,
            &self.account_provider_client,
            registration_data,
            &config.account_server.base_url,
            &config.account_server.instruction_result_public_key,
        );
        let remote_key_factory = RemoteEcdsaKeyFactory::new(&remote_instruction);

        let mdocs = self
            .pid_issuer
            .accept_pid(&config.mdoc_trust_anchors(), &remote_key_factory)
            .await
            .map_err(|error| {
                match error {
                    // We knowingly call unwrap() on the downcast to [RemoteEcdsaKeyError] here because we know that is the error type of the [RemoteEcdsaKeyFactory] we provide above
                    PidIssuerError::MdocError(nl_wallet_mdoc::Error::KeyGeneration(error)) => {
                        match *error.downcast::<RemoteEcdsaKeyError>().unwrap() {
                            RemoteEcdsaKeyError::Instruction(error) => PidIssuanceError::Instruction(error),
                            RemoteEcdsaKeyError::Signature(error) => PidIssuanceError::Signature(error),
                            RemoteEcdsaKeyError::KeyNotFound(identifier) => PidIssuanceError::KeyNotFound(identifier),
                        }
                    }
                    _ => PidIssuanceError::PidIssuer(error),
                }
            })?;

        self.storage.get_mut().insert_mdocs(mdocs).await?;

        self.emit_documents().await?;

        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn reject_pid_issuance(&mut self) -> Result<(), PidIssuanceError> {
        self.pid_issuer.reject_pid().await.map_err(PidIssuanceError::PidIssuer)
    }
}

#[cfg(test)]
mod tests {
    use wallet_common::keys::software::SoftwareEcdsaKey;

    use crate::{
        account_provider::MockAccountProviderClient, config::LocalConfigurationRepository, digid::MockDigidSession,
        pid_issuer::MockPidIssuerClient, storage::MockStorage,
    };

    use super::*;

    type MockWallet = Wallet<
        LocalConfigurationRepository,
        MockStorage,
        SoftwareEcdsaKey,
        MockAccountProviderClient,
        MockDigidSession,
        MockPidIssuerClient,
    >;

    // Create mocks and call wallet:init_registration(), with the option to override the mock storage.
    async fn init_wallet(storage: Option<MockStorage>) -> Result<MockWallet, WalletInitError> {
        let storage = storage.unwrap_or_default();

        Wallet::init_registration(
            LocalConfigurationRepository::default(),
            storage,
            MockAccountProviderClient::default(),
            MockPidIssuerClient::default(),
        )
        .await
    }

    // TODO: Add more unit tests for `Wallet`, using its mock dependencies.

    // Tests if the Wallet::init() method completes successfully with the mock generics.
    #[tokio::test]
    async fn test_init() {
        let wallet = init_wallet(None).await.expect("Could not initialize wallet");

        assert!(!wallet.has_registration());
    }

    // Tests the logic of fetching the wallet registration during init and its interaction with the database.
    #[tokio::test]
    async fn test_init_fetch_registration() {
        // Test with a wallet without a database file.
        let wallet = init_wallet(None).await.expect("Could not initialize wallet");

        // The wallet should have no registration, and no database should be opened.
        assert!(wallet.registration.is_none());
        assert!(!wallet.has_registration());
        assert!(matches!(
            wallet.storage.read().await.state().await.unwrap(),
            StorageState::Uninitialized
        ));

        // The wallet should be locked by default
        assert!(wallet.is_locked());

        // Test with a wallet with a database file, no registration.
        let wallet = init_wallet(Some(MockStorage::mock(StorageState::Unopened, None)))
            .await
            .expect("Could not initialize wallet");

        // The wallet should have no registration, the database should be opened.
        assert!(wallet.registration.is_none());
        assert!(!wallet.has_registration());
        assert!(matches!(
            wallet.storage.read().await.state().await.unwrap(),
            StorageState::Opened
        ));

        // Test with a wallet with a database file, contains registration.
        let pin_salt = new_pin_salt();
        let wallet = init_wallet(Some(MockStorage::mock(
            StorageState::Unopened,
            Some(RegistrationData {
                pin_salt: pin_salt.clone().into(),
                wallet_certificate: "thisisjwt".to_string().into(),
            }),
        )))
        .await
        .expect("Could not initialize wallet");

        // The wallet should have a registration, the database should be opened.
        assert!(wallet.registration.is_some());
        assert!(wallet.has_registration());
        assert!(matches!(
            wallet.storage.read().await.state().await.unwrap(),
            StorageState::Opened
        ));

        // The registration data should now be available.
        assert_eq!(wallet.registration.unwrap().pin_salt.0, pin_salt);
    }
}
