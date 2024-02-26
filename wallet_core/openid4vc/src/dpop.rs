//! Implements the DPoP HTTP header as in https://datatracker.ietf.org/doc/html/rfc9449.
//!
//! Like PKCE, DPoP allows a HTTP server to authenticate a HTTP client as the same client of some previous request.
//! However, being based on asymmetric signature schemes it can be used across more than two HTTP requests,
//! unlike PKCE, and additionally it can also sign other things such as (access) tokens.
//!
//! A DPoP is a JWT which (1) includes the public key with which it can be verified, and (2) signs the URL and HTTP
//! method of the HTTP request being done. In a second HTTP request, the HTTP server can authenticate the request
//! by matching the public key against that of the DPoP JWT from some earlier request, and verifying the signature.
//!
//! HTTP endpoints that require some token (e.g. an (access) token as in OpenID/OAuth) may additionally require this
//! token to be signed in a DPoP JWT, to prevent replay attacks. If so, then:
//! - the token itself must be sent as a HTTP header as follows:
//!   ```text
//!   Authorization: DPoP $token
//!   ```
//! - The DPoP JWT must include the `ath` field in its body, which must be equal to the URL-safe-no-pad base64 encoding
//!   of the SHA256 of the token.
//!
//! Example DPoP JWT header and body:
//! ```json
//! {
//!   "typ": "dpop+jwt",
//!   "alg": "ES256",
//!   "jwk": {
//!     "kty": "EC",
//!     "crv": "P-256"
//!     "x": "l8tFrhx-34tV3hRICRDY9zCkDlpBhF42UQUfWVAWBFs",
//!     "y": "9VE4jf_Ok_o64zbTTlcuNJajHmt6v9TDVrU0CdvGRDA",
//!   }
//! }
//! .
//! {
//!   "jti": "-BwC3ESc6acc2lTc",
//!   "htm": "POST",
//!   "htu": "https://server.example.com/token",
//!   "iat": 1562262616
//! }
//! ```

use chrono::{serde::ts_seconds, DateTime, Utc};
use jsonwebtoken::{Algorithm, TokenData, Validation};
use p256::ecdsa::VerifyingKey;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_with::{
    base64::{Base64, UrlSafe},
    formats::Unpadded,
    serde_as, skip_serializing_none,
};
use url::Url;
use wallet_common::{
    jwt::{EcdsaDecodingKey, Jwt, JwtError},
    keys::EcdsaKey,
    utils::random_string,
};

use crate::{
    jwt::{jwk_jwt_header, jwk_to_p256, JwkConversionError},
    token::AccessToken,
};

pub const DPOP_HEADER_NAME: &str = "DPoP";
pub const DPOP_NONCE_HEADER_NAME: &str = "DPoP-Nonce";

#[derive(Debug, thiserror::Error)]
pub enum DpopError {
    #[error("unsupported JWT algorithm: expected {}, found {}", expected, found.as_ref().unwrap_or(&"<None>".to_string()))]
    UnsupportedJwtAlgorithm { expected: String, found: Option<String> },
    #[error("incorrect DPoP JWT HTTP method")]
    IncorrectMethod,
    #[error("incorrect DPoP JWT url")]
    IncorrectUrl,
    #[error("incorrect DPoP JWT nonce")]
    IncorrectNonce,
    #[error("incorrect DPoP JWT access token hash")]
    IncorrectAccessTokenHash,
    #[error("missing JWK")]
    MissingJwk,
    #[error("incorrect JWK public key")]
    IncorrectJwkPublicKey,
    #[error("failed to convert key from/to JWK format: {0}")]
    JwkConversion(#[from] JwkConversionError),
    #[error("JWT decoding failed: {0}")]
    JwtDecodingFailed(#[from] jsonwebtoken::errors::Error),
    #[error("JWT error: {0}")]
    Jwt(#[from] JwtError),
}

pub type Result<T, E = DpopError> = std::result::Result<T, E>;

#[serde_as]
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DpopPayload {
    #[serde(rename = "htu")]
    http_url: Url,
    #[serde(rename = "htm")]
    http_method: String,
    #[serde(rename = "ath")]
    #[serde_as(as = "Option<Base64<UrlSafe, Unpadded>>")]
    access_token_hash: Option<Vec<u8>>,
    nonce: Option<String>,
    jti: String,
    #[serde(with = "ts_seconds")]
    iat: DateTime<Utc>,
}

#[derive(Clone)]
pub struct Dpop(Jwt<DpopPayload>);

impl AsRef<str> for Dpop {
    fn as_ref(&self) -> &str {
        &self.0 .0
    }
}

impl From<String> for Dpop {
    fn from(value: String) -> Self {
        Self(value.into())
    }
}

pub const OPENID4VCI_DPOP_JWT_TYPE: &str = "dpop+jwt";

impl Dpop {
    pub async fn new(
        private_key: &impl EcdsaKey,
        url: Url,
        method: Method,
        access_token: Option<&AccessToken>,
        nonce: Option<String>,
    ) -> Result<Self> {
        let header = jwk_jwt_header(OPENID4VCI_DPOP_JWT_TYPE, private_key).await?;

        let payload = DpopPayload {
            jti: random_string(32),
            iat: Utc::now(),
            http_method: method.to_string(),
            http_url: url,
            nonce,
            access_token_hash: access_token.map(AccessToken::sha256),
        };

        let jwt = Jwt::sign(&payload, &header, private_key).await?;
        Ok(Self(jwt))
    }

    fn verify_signature(&self, verifying_key: &VerifyingKey) -> Result<TokenData<DpopPayload>> {
        let mut validation_options = Validation::new(Algorithm::ES256);
        validation_options.required_spec_claims = Default::default();
        let token_data = jsonwebtoken::decode::<DpopPayload>(
            &self.0 .0,
            &EcdsaDecodingKey::from(verifying_key).0,
            &validation_options,
        )?;
        Ok(token_data)
    }

    fn verify_data(
        &self,
        token_data: &TokenData<DpopPayload>,
        url: &Url,
        method: &Method,
        access_token: Option<&AccessToken>,
        nonce: Option<&String>,
    ) -> Result<()> {
        if token_data.header.typ != Some(OPENID4VCI_DPOP_JWT_TYPE.to_string()) {
            return Err(DpopError::UnsupportedJwtAlgorithm {
                expected: OPENID4VCI_DPOP_JWT_TYPE.to_string(),
                found: token_data.header.typ.clone(),
            });
        }
        if token_data.claims.http_method != method.to_string() {
            return Err(DpopError::IncorrectMethod);
        }
        if token_data.claims.http_url != *url {
            return Err(DpopError::IncorrectUrl);
        }
        if token_data.claims.access_token_hash != access_token.map(AccessToken::sha256) {
            return Err(DpopError::IncorrectAccessTokenHash);
        }

        // We do not check the `jti` field to avoid having to keep track of this in the server state.
        // Verifying `jti` is not required by its spec (https://datatracker.ietf.org/doc/html/rfc9449).
        // We also do not check the `iat` field, to avoid having to deal with clockdrift.
        // Instead of both of these, the server can specify a `nonce` and later enforce its presence in the DPoP.
        if token_data.claims.nonce.as_ref() != nonce {
            return Err(DpopError::IncorrectNonce);
        }

        Ok(())
    }

    /// Verify the DPoP JWT against the public key inside its header, returning that public key.
    /// This should only be called in the first HTTP request of a protocol. In later requests,
    /// [`Dpop::verify_expecting_key()`] should be used with the public key that this method returns.
    pub async fn verify(&self, url: Url, method: Method, access_token: Option<&AccessToken>) -> Result<VerifyingKey> {
        // Grab the public key from the JWT header
        let header = jsonwebtoken::decode_header(&self.0 .0)?;
        let verifying_key = jwk_to_p256(&header.jwk.ok_or(DpopError::MissingJwk)?)?;

        let token_data = self.verify_signature(&verifying_key)?;
        self.verify_data(&token_data, &url, &method, access_token, None)?;

        Ok(verifying_key)
    }

    /// Verify the DPoP JWT against the specified public key obtained previously from [`Dpop::verify()`].
    pub async fn verify_expecting_key(
        &self,
        expected_verifying_key: &VerifyingKey,
        url: &Url,
        method: &Method,
        access_token: Option<&AccessToken>,
        nonce: Option<&String>,
    ) -> Result<()> {
        let token_data = self.verify_signature(expected_verifying_key)?;
        self.verify_data(&token_data, url, method, access_token, nonce)?;

        // Compare the specified key against the one in the JWT header
        let contained_key = jwk_to_p256(&token_data.header.jwk.ok_or(DpopError::MissingJwk)?)?;
        if contained_key != *expected_verifying_key {
            return Err(DpopError::IncorrectJwkPublicKey);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use base64::prelude::*;
    use jsonwebtoken::Header;
    use p256::{ecdsa::SigningKey, elliptic_curve::rand_core::OsRng};
    use reqwest::Method;
    use rstest::rstest;
    use serde::de::DeserializeOwned;
    use url::Url;

    use crate::{
        dpop::{DpopPayload, OPENID4VCI_DPOP_JWT_TYPE},
        token::AccessToken,
    };

    use super::Dpop;

    #[rstest]
    #[case(None, Some("123".to_string().into()))]
    #[case(Some("123".to_string().into()), None)]
    #[case(Some("123".to_string().into()), Some("456".to_string().into()))]
    #[tokio::test]
    async fn dpop(#[case] access_token: Option<AccessToken>, #[case] wrong_access_token: Option<AccessToken>) {
        let private_key = SigningKey::random(&mut OsRng);
        let url: Url = "https://example.com/path".parse().unwrap();
        let method = Method::POST;

        let dpop = Dpop::new(&private_key, url.clone(), method.clone(), access_token.as_ref(), None)
            .await
            .unwrap();

        // Check the `typ` of the Header
        let header: Header = part(0, &dpop.0 .0);
        assert_eq!(header.typ, Some(OPENID4VCI_DPOP_JWT_TYPE.to_string()));

        // Examine some fields in the claims
        let claims: DpopPayload = part(1, &dpop.0 .0);
        assert_eq!(claims.access_token_hash, access_token.as_ref().map(AccessToken::sha256));
        assert_eq!(claims.http_url, url);
        assert_eq!(claims.http_method, method.to_string());

        // Verifying it against incorrect parameters doesn't work
        dpop.verify(url.clone(), method.clone(), wrong_access_token.as_ref())
            .await
            .unwrap_err();
        dpop.verify(url.clone(), Method::PATCH, access_token.as_ref())
            .await
            .unwrap_err();
        dpop.verify(
            "https://incorrect_url/".parse().unwrap(),
            method.clone(),
            access_token.as_ref(),
        )
        .await
        .unwrap_err();

        // We can verify the DPoP
        let pubkey = dpop
            .verify(url.clone(), method.clone(), access_token.as_ref())
            .await
            .unwrap();
        assert_eq!(pubkey, *private_key.verifying_key());
        dpop.verify_expecting_key(&pubkey, &url, &method, access_token.as_ref(), None)
            .await
            .unwrap();
    }

    /// Decode and deserialize the specified part of the JWT.
    fn part<T: DeserializeOwned>(i: u8, jwt: &str) -> T {
        let bts = BASE64_URL_SAFE_NO_PAD
            .decode(jwt.split('.').take((i + 1).into()).last().unwrap())
            .unwrap();
        serde_json::from_slice(&bts).unwrap()
    }
}
