use futures::future::try_join_all;
use jsonwebtoken::{Algorithm, Header};
use nl_wallet_mdoc::utils::keys::{KeyFactory, MdocEcdsaKey};

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use url::Url;
use wallet_common::keys::SecureEcdsaKey;

use crate::{
    jwk_from_p256,
    jwt::{Jwt, StandardJwtClaims},
    Error, ErrorStatusCode, Format, Result,
};

/// https://openid.github.io/OpenID4VCI/openid-4-verifiable-credential-issuance-wg-draft.html#section-8.1.
/// Sent JSON-encoded to `POST /batch_credential`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CredentialRequests {
    pub credential_requests: Vec<CredentialRequest>,
}

/// https://openid.github.io/OpenID4VCI/openid-4-verifiable-credential-issuance-wg-draft.html#section-7.2.
/// Sent JSON-encoded to `POST /credential`.
// TODO: add `wallet_attestation`, `wallet_attestation_pop`, and `proof_of_secure_combination`
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CredentialRequest {
    pub format: Format,
    pub proof: CredentialRequestProof, // this is OPTIONAL per the spec, but we require it
}

/// https://openid.github.io/OpenID4VCI/openid-4-verifiable-credential-issuance-wg-draft.html#name-credential-endpoint
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "proof_type", rename_all = "snake_case")]
pub enum CredentialRequestProof {
    Jwt { jwt: Jwt<CredentialRequestProofJwtPayload> },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CredentialResponses {
    pub credential_responses: Vec<CredentialResponse>,
}

/// https://openid.github.io/OpenID4VCI/openid-4-verifiable-credential-issuance-wg-draft.html#name-credential-response.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CredentialResponse {
    pub format: Format,
    pub credential: serde_json::Value, // TODO maybe make this typed
}

// https://openid.github.io/OpenID4VCI/openid-4-verifiable-credential-issuance-wg-draft.html#section-7.2.1.1
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CredentialRequestProofJwtPayload {
    /// Required: `iss`, `aud`, `iat`
    #[serde(flatten)]
    pub jwt_claims: StandardJwtClaims,
    pub nonce: String,
}

pub const OPENID4VCI_VC_POP_JWT_TYPE: &str = "openid4vci-proof+jwt";

/// https://openid.github.io/OpenID4VCI/openid-4-verifiable-credential-issuance-wg-draft.html#name-credential-error-response
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum CredentialErrorType {
    InvalidCredentialRequest,
    UnsupportedCredentialType,
    UnsupportedCredentialFormat,
    InvalidProof,
    InvalidEncryptionParameters,
    ServerError,

    // From https://www.rfc-editor.org/rfc/rfc6750.html#section-3.1
    InvalidRequest,
    InvalidToken,
    InsufficientScope,
}

impl ErrorStatusCode for CredentialErrorType {
    fn status_code(&self) -> reqwest::StatusCode {
        match self {
            CredentialErrorType::InvalidCredentialRequest => StatusCode::BAD_REQUEST,
            CredentialErrorType::UnsupportedCredentialType => StatusCode::BAD_REQUEST,
            CredentialErrorType::UnsupportedCredentialFormat => StatusCode::BAD_REQUEST,
            CredentialErrorType::InvalidProof => StatusCode::BAD_REQUEST,
            CredentialErrorType::InvalidEncryptionParameters => StatusCode::BAD_REQUEST,
            CredentialErrorType::ServerError => StatusCode::INTERNAL_SERVER_ERROR,
            CredentialErrorType::InvalidRequest => StatusCode::BAD_REQUEST,
            CredentialErrorType::InvalidToken => StatusCode::UNAUTHORIZED,
            CredentialErrorType::InsufficientScope => StatusCode::FORBIDDEN,
        }
    }
}

impl CredentialRequestProof {
    pub async fn new_multiple<'a, K: MdocEcdsaKey + Sync>(
        nonce: String,
        wallet_client_id: String,
        credential_issuer_identifier: &Url,
        number_of_keys: u64,
        key_factory: &'a impl KeyFactory<'a, Key = K>,
    ) -> Result<Vec<(K, CredentialRequestProof)>> {
        // TODO: extend key factory so that it can do this in a single instruction
        let keys = key_factory.generate_new_multiple(number_of_keys).await.unwrap(); // TODO
        try_join_all(keys.into_iter().map(|privkey| async {
            CredentialRequestProof::new(
                &privkey,
                nonce.clone(),
                wallet_client_id.clone(),
                credential_issuer_identifier,
            )
            .await
            .map(|jwt| (privkey, jwt))
        }))
        .await
    }

    pub async fn new(
        private_key: &impl SecureEcdsaKey,
        nonce: String,
        wallet_client_id: String,
        credential_issuer_identifier: &Url,
    ) -> Result<Self> {
        let header = Header {
            typ: Some(OPENID4VCI_VC_POP_JWT_TYPE.to_string()),
            alg: Algorithm::ES256,
            jwk: Some(jwk_from_p256(
                &private_key
                    .verifying_key()
                    .await
                    .map_err(|e| Error::VerifyingKeyFromPrivateKey(e.into()))?,
            )?),
            ..Default::default()
        };

        let payload = CredentialRequestProofJwtPayload {
            jwt_claims: StandardJwtClaims {
                issuer: Some(wallet_client_id),
                audience: Some(credential_issuer_identifier.to_string()),
                issued_at: Some(jsonwebtoken::get_current_timestamp() as i64),
                ..Default::default()
            },
            nonce,
        };

        let jwt = Jwt::sign(&payload, &header, private_key).await?;

        Ok(CredentialRequestProof::Jwt { jwt })
    }
}
