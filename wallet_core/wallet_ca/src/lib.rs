use std::{fs, io, path::Path};

use anyhow::{anyhow, Result};
use clio::CachedInput;
use p256::{
    ecdsa::SigningKey,
    pkcs8::{DecodePrivateKey, EncodePrivateKey},
};
use pem::{EncodeConfig, LineEnding, Pem};

use nl_wallet_mdoc::utils::{reader_auth::ReaderRegistration, x509::Certificate};

pub fn read_certificate(input: CachedInput) -> Result<Certificate> {
    let input_string = io::read_to_string(input)?;
    let crt = Certificate::from_pem(&input_string)?;
    Ok(crt)
}

pub fn read_signing_key(input: CachedInput) -> Result<SigningKey> {
    let pem: Pem = io::read_to_string(input)?.parse()?;
    let key = SigningKey::from_pkcs8_der(pem.contents())?;
    Ok(key)
}

pub fn read_reader_registration(path: CachedInput) -> Result<ReaderRegistration> {
    let reader_registration = serde_json::from_reader(path)?;
    Ok(reader_registration)
}

pub fn write_key_pair(key: SigningKey, certificate: Certificate, file_prefix: &str, force: bool) -> Result<()> {
    // Verify certificate and key files do not exist before writing to either
    let crt_file = format!("{}.crt.pem", file_prefix);
    let crt_path = Path::new(&crt_file);
    assert_not_exists(crt_path, force)?;

    let key_file = format!("{}.key.pem", file_prefix);
    let key_path = Path::new(&key_file);
    assert_not_exists(key_path, force)?;

    write_certificate(crt_path, certificate)?;
    write_signing_key(key_path, key)?;

    Ok(())
}

fn assert_not_exists(file_path: &Path, force: bool) -> Result<()> {
    if file_path.exists() && !force {
        return Err(anyhow!("Target file '{}' already exists", file_path.display()));
    }
    Ok(())
}

fn write_certificate(file_path: &Path, certificate: Certificate) -> Result<()> {
    let crt_pem = Pem::new("CERTIFICATE", certificate.as_bytes());
    fs::write(
        file_path,
        pem::encode_config(&crt_pem, EncodeConfig::new().set_line_ending(LineEnding::LF)),
    )?;
    eprintln!("Certificate stored in '{}'", file_path.display());
    Ok(())
}

fn write_signing_key(file_path: &Path, key: SigningKey) -> Result<()> {
    let key_pkcs8_der = key.to_pkcs8_der()?;
    let key_pem = Pem::new("PRIVATE KEY", key_pkcs8_der.as_bytes());
    fs::write(
        file_path,
        pem::encode_config(&key_pem, EncodeConfig::new().set_line_ending(LineEnding::LF)),
    )?;
    eprintln!("Key stored in '{}'", file_path.display());
    Ok(())
}
