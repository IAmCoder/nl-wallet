use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use once_cell::sync::Lazy;
use p256::ecdsa::{signature::Signer, Signature, SigningKey, VerifyingKey};
use rand_core::{OsRng, RngCore};
use std::{collections::HashMap, sync::Mutex};
use wallet_shared::account::signing_key::{EcdsaKey, SecureEcdsaKey};

use super::{HardwareKeyStoreError, PlatformEcdsaKey, PlatformEncryptionKey};

// static for storing identifier -> signing key mapping, will only every grow
static SIGNING_KEYS: Lazy<Mutex<HashMap<String, SigningKey>>> = Lazy::new(|| Mutex::new(HashMap::new()));
// static for storing identifier -> encryption key mapping, will only ever grow
static ENCRYPTION_KEYS: Lazy<Mutex<HashMap<String, SoftwareEncryptionKey>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub struct SoftwareEcdsaKey(SigningKey);

impl From<SigningKey> for SoftwareEcdsaKey {
    fn from(value: SigningKey) -> Self {
        SoftwareEcdsaKey(value)
    }
}
impl Signer<Signature> for SoftwareEcdsaKey {
    fn try_sign(&self, msg: &[u8]) -> Result<Signature, p256::ecdsa::Error> {
        Signer::try_sign(&self.0, msg)
    }
}
impl EcdsaKey for SoftwareEcdsaKey {
    type Error = p256::ecdsa::Error;

    fn verifying_key(&self) -> Result<VerifyingKey, Self::Error> {
        Ok(*self.0.verifying_key())
    }
}
impl SecureEcdsaKey for SoftwareEcdsaKey {}

// SigningKey from p256::ecdsa conforms to the SigningKey trait
// if we provide an implementation for the signing_key and verifying_key methods.
impl PlatformEcdsaKey for SoftwareEcdsaKey {
    fn signing_key(identifier: &str) -> Result<Self, HardwareKeyStoreError> {
        // obtain lock on SIGNING_KEYS static hashmap
        let mut signing_keys = SIGNING_KEYS.lock().expect("Could not get lock on SIGNING_KEYS");
        // insert new random signing key, if the key is not present
        let key = signing_keys
            .entry(identifier.to_string())
            .or_insert_with(|| SigningKey::random(&mut OsRng));

        // make a clone of the (mutable) signing key so we can
        // return (non-mutable) ownership to the caller
        Ok(key.clone().into())
    }
}

#[derive(Clone)]
pub struct SoftwareEncryptionKey {
    cipher: Aes256Gcm,
}

impl PlatformEncryptionKey for SoftwareEncryptionKey {
    fn encryption_key(identifier: &str) -> Result<Self, HardwareKeyStoreError>
    where
        Self: Sized,
    {
        // obtain lock on ENCRYPTION_KEYS static hashmap
        let mut encryption_keys = ENCRYPTION_KEYS.lock().expect("Could not get lock on ENCRYPTION_KEYS");

        // insert new random signing key, if the key is not present
        let key = encryption_keys.entry(identifier.to_string()).or_insert_with(|| {
            let key = Aes256Gcm::generate_key(&mut OsRng);
            let cipher = Aes256Gcm::new(&key);
            SoftwareEncryptionKey { cipher }
        });

        // make a clone of the (mutable) signing key so we can
        // return (non-mutable) ownership to the caller
        Ok(key.clone())
    }

    fn encrypt(&self, msg: &[u8]) -> Result<Vec<u8>, HardwareKeyStoreError> {
        let cipher = &self.cipher;

        // Generate a random nonce
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes); // 96-bits; unique per message

        // Encrypt the provided message
        let encrypted_msg = cipher.encrypt(nonce, msg).expect("Could not encrypt message");

        // concatenate nonce with encrypted payload
        let result: Vec<_> = nonce_bytes.into_iter().chain(encrypted_msg).collect();

        Ok(result)
    }

    fn decrypt(&self, msg: &[u8]) -> Result<Vec<u8>, HardwareKeyStoreError> {
        let cipher = &self.cipher;

        // Re-create the nonce from the first 12 bytes
        let nonce = Nonce::from_slice(&msg[..12]);

        // Decrypt the provided message with the retrieved nonce
        let decrypted_msg = cipher.decrypt(nonce, &msg[12..]).expect("Could not decrypt message");

        Ok(decrypted_msg)
    }
}
