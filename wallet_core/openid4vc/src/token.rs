use std::{num::NonZero, time::Duration};

use derive_more::From;
use indexmap::IndexSet;

use serde::{Deserialize, Serialize};
use serde_with::{formats::SpaceSeparator, serde_as, skip_serializing_none, DurationSeconds, StringWithSeparator};
use url::Url;

use error_category::ErrorCategory;
use nl_wallet_mdoc::{
    holder::TrustAnchor,
    unsigned::UnsignedMdoc,
    utils::{
        issuer_auth::IssuerRegistration,
        x509::{Certificate, CertificateError, CertificateType, CertificateUsage},
    },
};
use wallet_common::{
    generator::TimeGenerator,
    nonempty::NonEmpty,
    utils::{random_string, sha256},
};

use crate::{authorization::AuthorizationDetails, jwt::JwtCredentialContents, server_state::SessionToken, Format};

#[derive(Serialize, Deserialize, Debug, Clone, From)]
pub struct AuthorizationCode(String);

impl AsRef<str> for AuthorizationCode {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, From)]
pub struct AccessToken(String);

impl AsRef<str> for AccessToken {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<AuthorizationCode> for SessionToken {
    fn from(value: AuthorizationCode) -> Self {
        SessionToken::from(value.0)
    }
}

impl AccessToken {
    /// Construct a new random access token, with the specified authorization code appended to it.
    pub(crate) fn new(code: &AuthorizationCode) -> Self {
        Self(random_string(32) + code.as_ref())
    }

    /// Returns the authorization code appended to this access token.
    pub(crate) fn code(&self) -> Option<AuthorizationCode> {
        self.as_ref().get(32..).map(|code| AuthorizationCode(code.to_string()))
    }

    pub(crate) fn sha256(&self) -> Vec<u8> {
        sha256(self.as_ref().as_bytes())
    }
}

/// https://openid.github.io/OpenID4VCI/openid-4-verifiable-credential-issuance-wg-draft.html#name-token-request
/// and https://www.rfc-editor.org/rfc/rfc6749.html#section-4.1.3.
/// Sent URL-encoded in request body to POST /token.
/// A DPoP HTTP header may be included.
#[skip_serializing_none]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenRequest {
    #[serde(flatten)]
    pub grant_type: TokenRequestGrantType,

    pub code_verifier: Option<String>,
    pub client_id: Option<String>,

    /// MUST be the redirect URI value as passed to the authorization request
    pub redirect_uri: Option<Url>,
}

impl TokenRequest {
    /// Retrieve either the authorization code or the pre-authorized code, depending on the authorization grant type.
    pub fn code(&self) -> &AuthorizationCode {
        match &self.grant_type {
            TokenRequestGrantType::AuthorizationCode { code } => code,
            TokenRequestGrantType::PreAuthorizedCode { pre_authorized_code } => pre_authorized_code,
        }
    }
}

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename = "snake_case")]
#[serde(tag = "grant_type")]
pub enum TokenRequestGrantType {
    #[serde(rename = "authorization_code")]
    AuthorizationCode { code: AuthorizationCode },
    #[serde(rename = "urn:ietf:params:oauth:grant-type:pre-authorized_code")]
    PreAuthorizedCode {
        #[serde(rename = "pre-authorized_code")]
        pre_authorized_code: AuthorizationCode,
    },
}

/// https://openid.github.io/OpenID4VCI/openid-4-verifiable-credential-issuance-wg-draft.html#name-successful-token-response
/// and https://www.rfc-editor.org/rfc/rfc6749.html#section-5.1
#[serde_as]
#[skip_serializing_none]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: AccessToken,
    pub token_type: TokenType,
    pub refresh_token: Option<String>,
    pub c_nonce: Option<String>,

    #[serde_as(as = "Option<StringWithSeparator::<SpaceSeparator, String>>")]
    pub scope: Option<IndexSet<String>>,

    #[serde_as(as = "Option<DurationSeconds<u64>>")]
    pub c_nonce_expires_in: Option<Duration>, // lifetime of `c_nonce`

    #[serde_as(as = "Option<DurationSeconds<u64>>")]
    pub expires_in: Option<Duration>,

    /// "REQUIRED when authorization_details parameter is used to request issuance of a certain Credential type
    /// as defined in Section 5.1.1. MUST NOT be used otherwise."
    pub authorization_details: Option<AuthorizationDetails>,
}

/// A [`TokenResponse`] with an extra field for the credential previews.
/// This is an custom field so other implementations might not send it. For now however we assume that it is always
/// present so it is not an [`Option`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenResponseWithPreviews {
    #[serde(flatten)]
    pub token_response: TokenResponse,
    pub credential_previews: NonEmpty<Vec<CredentialPreview>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "format", rename_all = "snake_case")]
pub enum CredentialPreview {
    MsoMdoc {
        unsigned_mdoc: UnsignedMdoc,
        issuer: Certificate,
    },
    Jwt {
        jwt_typ: Option<String>,
        claims: JwtCredentialContents,
        copy_count: NonZero<u8>,
    },
}

impl From<&CredentialPreview> for Format {
    fn from(value: &CredentialPreview) -> Self {
        match value {
            CredentialPreview::MsoMdoc { .. } => Format::MsoMdoc,
            CredentialPreview::Jwt { .. } => Format::Jwt,
        }
    }
}

impl CredentialPreview {
    pub fn copy_count(&self) -> u8 {
        match self {
            CredentialPreview::MsoMdoc { unsigned_mdoc, .. } => unsigned_mdoc.copy_count.into(),
            CredentialPreview::Jwt { copy_count, .. } => (*copy_count).into(),
        }
    }

    pub fn credential_type(&self) -> Option<&str> {
        match self {
            CredentialPreview::MsoMdoc { unsigned_mdoc, .. } => Some(&unsigned_mdoc.doc_type),
            CredentialPreview::Jwt { .. } => None,
        }
    }

    pub fn verify(&self, trust_anchors: &[TrustAnchor<'_>]) -> Result<(), CertificateError> {
        match self {
            CredentialPreview::MsoMdoc { issuer, .. } => {
                // Verify the issuer certificates that the issuer presents for each credential to be issued.
                // NB: this only proves the authenticity of the data inside the certificates (the [`IssuerRegistration`]s),
                // but does not authenticate the issuer that presents them.
                // Anyone that has ever seen these certificates (such as other wallets that received them during issuance)
                // could present them here in the protocol without needing the corresponding issuer private key.
                // This is not a problem, because at the end of the issuance protocol each mdoc is verified against the
                // corresponding certificate in the credential preview, which implicitly authenticates the issuer because
                // only it could have produced an mdoc against that certificate.
                issuer.verify(CertificateUsage::Mdl, &[], &TimeGenerator, trust_anchors)
            }
            CredentialPreview::Jwt { .. } => Ok(()),
        }
    }
}

#[derive(Debug, thiserror::Error, ErrorCategory)]
pub enum CredentialPreviewError {
    #[error("certificate error: {0}")]
    #[category(defer)]
    Certificate(#[from] CertificateError),
    #[error("issuer registration not found in certificate")]
    #[category(critical)]
    NoIssuerRegistration,
    #[error("unexpected credential format: expected MsoMdoc, found {0:?}")]
    #[category(critical)]
    UnexpectedFormat(Format),
}

impl TryFrom<CredentialPreview> for (UnsignedMdoc, Box<IssuerRegistration>) {
    type Error = CredentialPreviewError;

    fn try_from(value: CredentialPreview) -> Result<Self, Self::Error> {
        let CredentialPreview::MsoMdoc { unsigned_mdoc, issuer } = value else {
            Err(CredentialPreviewError::UnexpectedFormat(Format::Jwt))?
        };
        let CertificateType::Mdl(Some(issuer)) = CertificateType::from_certificate(&issuer)? else {
            Err(CredentialPreviewError::NoIssuerRegistration)?
        };
        Ok((unsigned_mdoc, issuer))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub enum TokenType {
    #[default]
    Bearer,
    DPoP,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use indexmap::IndexSet;

    use crate::token::{TokenRequest, TokenRequestGrantType, TokenResponse};

    #[test]
    fn token_request_serialization() {
        assert_eq!(
            serde_urlencoded::to_string(TokenRequest {
                grant_type: TokenRequestGrantType::PreAuthorizedCode {
                    pre_authorized_code: "123".to_string().into()
                },
                code_verifier: Some("myverifier".to_string()),
                client_id: Some("myclient".to_string()),
                redirect_uri: Some("https://example.com".parse().unwrap())
            })
            .unwrap(),
            "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Apre-authorized_code&pre-authorized_code=123&code_verifier=myverifier&client_id=myclient&redirect_uri=https%3A%2F%2Fexample.com%2F",
        )
    }

    #[test]
    fn token_response_serialization() {
        assert_eq!(
            serde_json::to_string(&TokenResponse {
                access_token: "access_token".to_string().into(),
                token_type: crate::token::TokenType::Bearer,
                c_nonce: Some("c_nonce".to_string()),
                scope: Some(IndexSet::from_iter(["scope1".to_string(), "scope2".to_string()])),
                c_nonce_expires_in: Some(Duration::from_secs(10)),
                expires_in: None,
                refresh_token: None,
                authorization_details: None,
            })
            .unwrap(),
            r#"{"access_token":"access_token","token_type":"Bearer","c_nonce":"c_nonce","scope":"scope1 scope2","c_nonce_expires_in":10}"#.to_string(),
        )
    }
}
