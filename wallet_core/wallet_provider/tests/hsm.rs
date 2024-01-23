use std::sync::Arc;

use p256::ecdsa::{
    signature::{rand_core::OsRng, Verifier},
    SigningKey,
};
use serial_test::serial;

use wallet_common::utils::{random_bytes, random_string};
use wallet_provider::settings::Settings;
use wallet_provider_domain::model::{
    encrypted::Encrypted,
    encrypter::{Decrypter, Encrypter},
    hsm::{Hsm, WalletUserHsm},
    wallet_user::WalletId,
};
use wallet_provider_service::hsm::Pkcs11Hsm;

fn setup_hsm() -> (Pkcs11Hsm, Settings) {
    let settings = Settings::new().unwrap();
    let hsm = Pkcs11Hsm::new(
        settings.hsm.library_path,
        settings.hsm.user_pin,
        settings.hsm.max_sessions,
        settings.hsm.max_session_lifetime,
        settings.attestation_wrapping_key_identifier,
    )
    .unwrap();
    (hsm, Settings::new().unwrap())
}

#[tokio::test]
#[serial]
async fn generate_key_and_sign() {
    let (hsm, _) = setup_hsm();

    let wallet_id: WalletId = String::from("wallet_user_1");
    let identifier = random_string(8);
    let public_key = hsm.generate_key(&wallet_id, &identifier).await.unwrap();

    let data = Arc::new(random_bytes(32));
    let signature = WalletUserHsm::sign(&hsm, &wallet_id, &identifier, Arc::clone(&data))
        .await
        .unwrap();
    public_key.verify(data.as_ref(), &signature).unwrap();

    Hsm::delete_key(&hsm, &format!("{wallet_id}_{identifier}"))
        .await
        .unwrap();
}

#[tokio::test]
#[serial]
async fn sign_sha256_hmac_using_new_secret_key() {
    let (hsm, _) = setup_hsm();

    let secret_key = "generic_secret_key_1";
    let data = Arc::new(random_bytes(32));

    hsm.generate_generic_secret_key(secret_key).await.unwrap();

    let signature = hsm.sign_hmac(secret_key, Arc::clone(&data)).await.unwrap();

    hsm.verify_hmac(secret_key, Arc::clone(&data), signature).await.unwrap();
}

#[tokio::test]
#[serial]
async fn sign_sha256_hmac() {
    let (hsm, settings) = setup_hsm();

    let data = Arc::new(random_bytes(32));

    let signature = hsm
        .sign_hmac(
            &settings.pin_public_disclosure_protection_key_identifier,
            Arc::clone(&data),
        )
        .await
        .unwrap();

    hsm.verify_hmac(
        &settings.pin_public_disclosure_protection_key_identifier,
        Arc::clone(&data),
        signature,
    )
    .await
    .unwrap();
}

#[tokio::test]
#[serial]
async fn wrap_key_and_sign() {
    let (hsm, _) = setup_hsm();

    let (public_key, wrapped) = hsm.generate_wrapped_key().await.unwrap();

    let data = Arc::new(random_bytes(32));
    let signature = WalletUserHsm::sign_wrapped(&hsm, wrapped, Arc::clone(&data))
        .await
        .unwrap();

    public_key.verify(data.as_ref(), &signature).unwrap();
}

#[tokio::test]
#[serial]
async fn encrypt_decrypt() {
    let (hsm, settings) = setup_hsm();

    let data = random_bytes(32);
    let encrypted: Encrypted<Vec<u8>> =
        Hsm::encrypt(&hsm, &settings.pin_pubkey_encryption_key_identifier, data.clone())
            .await
            .unwrap();

    assert_ne!(data.clone(), encrypted.data.clone());

    let decrypted = Hsm::decrypt(&hsm, &settings.pin_pubkey_encryption_key_identifier, encrypted)
        .await
        .unwrap();

    assert_eq!(data, decrypted);
}

#[tokio::test]
#[serial]
async fn encrypt_decrypt_verifying_key() {
    let (hsm, settings) = setup_hsm();

    let verifying_key = *SigningKey::random(&mut OsRng).verifying_key();
    let encrypted = Encrypter::encrypt(&hsm, &settings.pin_pubkey_encryption_key_identifier, verifying_key)
        .await
        .unwrap();

    let decrypted = Decrypter::decrypt(&hsm, &settings.pin_pubkey_encryption_key_identifier, encrypted)
        .await
        .unwrap();

    assert_eq!(verifying_key, decrypted);
}
