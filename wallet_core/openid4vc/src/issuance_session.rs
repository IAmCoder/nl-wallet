use std::collections::VecDeque;

use futures::{future::try_join_all, TryFutureExt};
use itertools::Itertools;
use p256::{
    ecdsa::{SigningKey, VerifyingKey},
    elliptic_curve::rand_core::OsRng,
};
use reqwest::{
    header::{ToStrError, AUTHORIZATION},
    Method,
};
use url::Url;

use nl_wallet_mdoc::{
    holder::{IssuedAttributesMismatch, Mdoc, MdocCopies, TrustAnchor},
    utils::{
        cose::CoseError,
        keys::{KeyFactory, MdocEcdsaKey},
        serialization::CborError,
        x509::{Certificate, CertificateError, CertificateUsage},
    },
    ATTR_RANDOM_LENGTH,
};
use wallet_common::{config::wallet_config::BaseUrl, generator::TimeGenerator, jwt::JwtError};

use crate::{
    credential::{
        CredentialErrorCode, CredentialRequest, CredentialRequestProof, CredentialRequests, CredentialResponse,
        CredentialResponses,
    },
    dpop::{Dpop, DpopError, DPOP_HEADER_NAME, DPOP_NONCE_HEADER_NAME},
    jwt::JwkConversionError,
    metadata::IssuerMetadata,
    oidc,
    token::{AccessToken, AttestationPreview, TokenErrorCode, TokenRequest, TokenResponseWithPreviews},
    ErrorResponse, Format, NL_WALLET_CLIENT_ID,
};

#[derive(Debug, thiserror::Error)]
pub enum IssuanceSessionError {
    #[error("failed to get public key: {0}")]
    VerifyingKeyFromPrivateKey(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("DPoP error: {0}")]
    Dpop(#[from] DpopError),
    #[error("failed to convert key from/to JWK format: {0}")]
    JwkConversion(#[from] JwkConversionError),
    #[error("JWT error: {0}")]
    Jwt(#[from] JwtError),
    #[error("http request failed: {0}")]
    Network(#[from] reqwest::Error),
    #[error("missing c_nonce")]
    MissingNonce,
    #[error("CBOR (de)serialization error: {0}")]
    Cbor(#[from] CborError),
    #[error("base64 decoding failed: {0}")]
    Base64Error(#[from] base64::DecodeError),
    #[error("mismatch between issued and expected attributes: {0}")]
    IssuedAttributesMismatch(IssuedAttributesMismatch),
    #[error("mdoc verification failed: {0}")]
    MdocVerification(#[source] nl_wallet_mdoc::Error),
    #[error("error requesting access token: {0:?}")]
    TokenRequest(Box<ErrorResponse<TokenErrorCode>>),
    #[error("error requesting credentials: {0:?}")]
    CredentialRequest(Box<ErrorResponse<CredentialErrorCode>>),
    #[error("generating attestation private keys failed: {0}")]
    PrivateKeyGeneration(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
    #[error("public key contained in mdoc not equal to expected value")]
    PublicKeyMismatch,
    #[error("failed to get mdoc public key: {0}")]
    PublicKeyFromMdoc(#[source] nl_wallet_mdoc::Error),
    #[error("received {found} responses, expected {expected}")]
    UnexpectedCredentialResponseCount { found: usize, expected: usize },
    #[error("error reading HTTP error: {0}")]
    HeaderToStr(#[from] ToStrError),
    #[error("error verifying certificate of attestation preview: {0}")]
    Certificate(#[from] CertificateError),
    #[error("issuer certificate contained in mdoc not equal to expected value")]
    IssuerCertificateMismatch,
    #[error("error retrieving issuer certificate from issued mdoc: {0}")]
    Cose(#[from] CoseError),
    #[error("error discovering Oauth metadata: {0}")]
    OauthDiscovery(#[source] reqwest::Error),
    #[error("error discovering OpenID4VCI Credential Issuer metadata: {0}")]
    OpenId4vciDiscovery(#[source] reqwest::Error),
    #[error("issuer has no batch credential endpoint")]
    NoBatchCredentialEndpoint,
    #[error("malformed attribute: random too short (was {0}; minimum {1}")]
    AttributeRandomLength(usize, usize),
}

pub trait IssuanceSession<H = HttpOpenidMessageClient> {
    async fn start_issuance(
        message_client: H,
        base_url: BaseUrl,
        token_request: TokenRequest,
        trust_anchors: &[TrustAnchor<'_>],
    ) -> Result<(Self, Vec<AttestationPreview>), IssuanceSessionError>
    where
        Self: Sized;

    async fn accept_issuance<K: MdocEcdsaKey>(
        &self,
        mdoc_trust_anchors: &[TrustAnchor<'_>],
        key_factory: impl KeyFactory<Key = K>,
        credential_issuer_identifier: BaseUrl,
    ) -> Result<Vec<MdocCopies>, IssuanceSessionError>;

    async fn reject_issuance(self) -> Result<(), IssuanceSessionError>;
}

pub struct HttpIssuanceSession<H = HttpOpenidMessageClient> {
    message_client: H,
    session_state: IssuanceState,
}

/// Contract for sending OpenID4VCI protocol messages.
pub trait OpenidMessageClient {
    async fn discover_metadata(&self, url: &BaseUrl) -> Result<IssuerMetadata, IssuanceSessionError>;
    async fn discover_oauth_metadata(&self, url: &BaseUrl) -> Result<oidc::Config, IssuanceSessionError>;

    async fn request_token(
        &self,
        url: &Url,
        token_request: &TokenRequest,
        dpop_header: &Dpop,
    ) -> Result<(TokenResponseWithPreviews, Option<String>), IssuanceSessionError>;

    async fn request_credentials(
        &self,
        url: &Url,
        credential_requests: &CredentialRequests,
        dpop_header: &str,
        access_token_header: &str,
    ) -> Result<CredentialResponses, IssuanceSessionError>;

    async fn reject(&self, url: &Url, dpop_header: &str, access_token_header: &str)
        -> Result<(), IssuanceSessionError>;
}

pub struct HttpOpenidMessageClient {
    http_client: reqwest::Client,
}

impl From<reqwest::Client> for HttpOpenidMessageClient {
    fn from(http_client: reqwest::Client) -> Self {
        Self { http_client }
    }
}

impl OpenidMessageClient for HttpOpenidMessageClient {
    async fn discover_metadata(&self, url: &BaseUrl) -> Result<IssuerMetadata, IssuanceSessionError> {
        let metadata = IssuerMetadata::discover(&self.http_client, url)
            .await
            .map_err(IssuanceSessionError::OpenId4vciDiscovery)?;
        Ok(metadata)
    }

    async fn discover_oauth_metadata(&self, url: &BaseUrl) -> Result<oidc::Config, IssuanceSessionError> {
        let metadata = self
            .http_client
            .get(url.join("/.well-known/oauth-authorization-server"))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .map_err(IssuanceSessionError::OauthDiscovery)?;
        Ok(metadata)
    }

    async fn request_token(
        &self,
        url: &Url,
        token_request: &TokenRequest,
        dpop_header: &Dpop,
    ) -> Result<(TokenResponseWithPreviews, Option<String>), IssuanceSessionError> {
        self.http_client
            .post(url.as_ref())
            .header(DPOP_HEADER_NAME, dpop_header.as_ref())
            .form(&token_request)
            .send()
            .map_err(IssuanceSessionError::from)
            .and_then(|response| async {
                // If the HTTP response code is 4xx or 5xx, parse the JSON as an error
                let status = response.status();
                if status.is_client_error() || status.is_server_error() {
                    let error = response.json::<ErrorResponse<TokenErrorCode>>().await?;
                    Err(IssuanceSessionError::TokenRequest(error.into()))
                } else {
                    let dpop_nonce = response
                        .headers()
                        .get(DPOP_NONCE_HEADER_NAME)
                        .map(|val| val.to_str())
                        .transpose()?
                        .map(str::to_string);
                    let deserialized = response.json::<TokenResponseWithPreviews>().await?;
                    Ok((deserialized, dpop_nonce))
                }
            })
            .await
    }

    async fn request_credentials(
        &self,
        url: &Url,
        credential_requests: &CredentialRequests,
        dpop_header: &str,
        access_token_header: &str,
    ) -> Result<CredentialResponses, IssuanceSessionError> {
        self.http_client
            .post(url.as_ref())
            .header(DPOP_HEADER_NAME, dpop_header)
            .header(AUTHORIZATION, access_token_header)
            .json(credential_requests)
            .send()
            .map_err(IssuanceSessionError::from)
            .and_then(|response| async {
                // If the HTTP response code is 4xx or 5xx, parse the JSON as an error
                let status = response.status();
                if status.is_client_error() || status.is_server_error() {
                    let error = response.json::<ErrorResponse<CredentialErrorCode>>().await?;
                    Err(IssuanceSessionError::CredentialRequest(error.into()))
                } else {
                    let credential_responses = response.json().await?;
                    Ok(credential_responses)
                }
            })
            .await
    }

    async fn reject(
        &self,
        url: &Url,
        dpop_header: &str,
        access_token_header: &str,
    ) -> Result<(), IssuanceSessionError> {
        self.http_client
            .delete(url.as_ref())
            .header(DPOP_HEADER_NAME, dpop_header)
            .header(AUTHORIZATION, access_token_header)
            .send()
            .map_err(IssuanceSessionError::from)
            .and_then(|response| async {
                // If the HTTP response code is 4xx or 5xx, parse the JSON as an error
                let status = response.status();
                if status.is_client_error() || status.is_server_error() {
                    let error = response.json::<ErrorResponse<CredentialErrorCode>>().await?;
                    Err(IssuanceSessionError::CredentialRequest(error.into()))
                } else {
                    Ok(())
                }
            })
            .await?;
        Ok(())
    }
}

struct IssuanceState {
    access_token: AccessToken,
    c_nonce: String,
    attestation_previews: Vec<AttestationPreview>,
    issuer_url: BaseUrl,
    dpop_private_key: SigningKey,
    dpop_nonce: Option<String>,
}

impl<H: OpenidMessageClient> HttpIssuanceSession<H> {
    /// Discover the token endpoint from the OAuth server metadata.
    async fn discover_token_endpoint(message_client: &H, base_url: &BaseUrl) -> Result<Url, IssuanceSessionError> {
        let issuer_metadata = message_client.discover_metadata(base_url).await?;

        // The issuer may announce multiple OAuth authorization servers the wallet may use. Which one the wallet
        // uses is left up to the wallet. We just take the first one.
        // authorization_servers() always returns a non-empty vec so the unwrap() is safe.
        let authorization_servers = &issuer_metadata.issuer_config.authorization_servers();
        let oauth_server = authorization_servers.first().unwrap();
        let oauth_metadata = message_client.discover_oauth_metadata(oauth_server).await?;

        let token_endpoint = oauth_metadata.token_endpoint.clone();
        Ok(token_endpoint)
    }

    /// Discover the batch credential endpoint from the Credential Issuer metadata.
    /// This function returns an `Option` because the batch credential is optional.
    async fn discover_batch_credential_endpoint(
        message_client: &H,
        base_url: &BaseUrl,
    ) -> Result<Option<Url>, IssuanceSessionError> {
        let url = message_client
            .discover_metadata(base_url)
            .await?
            .issuer_config
            .batch_credential_endpoint
            .map(|url| url.as_ref().clone());
        Ok(url)
    }
}

impl<H: OpenidMessageClient> IssuanceSession<H> for HttpIssuanceSession<H> {
    async fn start_issuance(
        message_client: H,
        base_url: BaseUrl,
        token_request: TokenRequest,
        trust_anchors: &[TrustAnchor<'_>],
    ) -> Result<(Self, Vec<AttestationPreview>), IssuanceSessionError> {
        let token_endpoint = Self::discover_token_endpoint(&message_client, &base_url).await?;

        let dpop_private_key = SigningKey::random(&mut OsRng);
        let dpop_header = Dpop::new(&dpop_private_key, token_endpoint.clone(), Method::POST, None, None).await?;

        let (token_response, dpop_nonce) = message_client
            .request_token(&token_endpoint, &token_request, &dpop_header)
            .await?;

        // Verify the issuer certificates that the issuer presents for each attestation to be issued.
        // NB: this only proves the authenticity of the data inside the certificates (the [`IssuerRegistration`]s),
        // but does not authenticate the issuer that presents them.
        // Anyone that has ever seen these certificates (such as other wallets that received them during issuance)
        // could present them here in the protocol without needing the corresponding issuer private key.
        // This is not a problem, because at the end of the issuance protocol each mdoc is verified against the
        // corresponding certificate in the attestation preview, which implicitly authenticates the issuer because
        // only it could have produced an mdoc against that certificate.
        token_response.attestation_previews.iter().try_for_each(|preview| {
            let issuer: &Certificate = preview.as_ref();
            issuer.verify(CertificateUsage::Mdl, &[], &TimeGenerator, trust_anchors)
        })?;

        // TODO: Check that each `UnsignedMdoc` contains at least one attribute (PVW-2546).
        let attestation_previews = token_response.attestation_previews.into_inner();

        let session_state = IssuanceState {
            access_token: token_response.token_response.access_token,
            c_nonce: token_response
                .token_response
                .c_nonce
                .ok_or(IssuanceSessionError::MissingNonce)?,
            attestation_previews: attestation_previews.clone(),
            issuer_url: base_url,
            dpop_private_key,
            dpop_nonce,
        };

        let issuance_client = Self {
            message_client,
            session_state,
        };
        Ok((issuance_client, attestation_previews))
    }

    async fn accept_issuance<K: MdocEcdsaKey>(
        &self,
        trust_anchors: &[TrustAnchor<'_>],
        key_factory: impl KeyFactory<Key = K>,
        credential_issuer_identifier: BaseUrl,
    ) -> Result<Vec<MdocCopies>, IssuanceSessionError> {
        // The OpenID4VCI `/batch_credential` endpoints supports issuance of multiple attestations, but the protocol
        // has no support (yet) for issuance of multiple copies of multiple attestations.
        // We implement this below by simply flattening the relevant nested iterators when communicating with the issuer.

        let doctypes = self
            .session_state
            .attestation_previews
            .iter()
            .flat_map(|preview| {
                itertools::repeat_n(
                    match preview {
                        AttestationPreview::MsoMdoc { unsigned_mdoc, .. } => unsigned_mdoc.doctype.clone(),
                    },
                    preview.copy_count().into(),
                )
            })
            .collect_vec();

        // Generate the PoPs to be sent to the issuer, and the private keys with which they were generated
        // (i.e., the private key of the future mdoc).
        // If N is the total amount of copies of attestations to be issued, then this returns N key/proof pairs.
        let keys_and_proofs = CredentialRequestProof::new_multiple(
            self.session_state.c_nonce.clone(),
            NL_WALLET_CLIENT_ID.to_string(),
            credential_issuer_identifier,
            doctypes.len().try_into().unwrap(),
            key_factory,
        )
        .await?;

        // Split into N keys and N credential requests, so we can send the credential request proofs separately
        // to the issuer.
        let (pubkeys, credential_requests): (Vec<_>, Vec<_>) = try_join_all(
            keys_and_proofs
                .into_iter()
                .zip(doctypes)
                .map(|((key, response), doctype)| async move {
                    let pubkey = key
                        .verifying_key()
                        .await
                        .map_err(|e| IssuanceSessionError::VerifyingKeyFromPrivateKey(e.into()))?;
                    let id = key.identifier().to_string();
                    let cred_request = CredentialRequest {
                        format: Format::MsoMdoc,
                        doctype: Some(doctype),
                        proof: Some(response),
                    };
                    Ok::<_, IssuanceSessionError>(((pubkey, id), cred_request))
                }),
        )
        .await?
        .into_iter()
        .unzip();

        let url = Self::discover_batch_credential_endpoint(&self.message_client, &self.session_state.issuer_url)
            .await?
            .ok_or(IssuanceSessionError::NoBatchCredentialEndpoint)?;
        let (dpop_header, access_token_header) = self.session_state.auth_headers(url.clone(), Method::POST).await?;

        let responses = self
            .message_client
            .request_credentials(
                &url,
                &CredentialRequests {
                    // This `.unwrap()` is safe as long as the received
                    // `TokenResponseWithPreviews.attestation_previews` is not empty.
                    credential_requests: credential_requests.try_into().unwrap(),
                },
                &dpop_header,
                &access_token_header,
            )
            .await?;

        // The server must have responded with enough credential responses, N, so that we have exactly enough responses
        // for all copies of all mdocs constructed below.
        if responses.credential_responses.len() != pubkeys.len() {
            return Err(IssuanceSessionError::UnexpectedCredentialResponseCount {
                found: responses.credential_responses.len(),
                expected: pubkeys.len(),
            });
        }

        let mut responses_and_pubkeys: VecDeque<_> = responses.credential_responses.into_iter().zip(pubkeys).collect();

        let mdocs = self
            .session_state
            .attestation_previews
            .iter()
            .map(|preview| {
                let copy_count: usize = preview.copy_count().into();

                // Consume the amount of copies from the front of `responses_and_keys`.
                let cred_copies = responses_and_pubkeys
                    .drain(..copy_count)
                    .map(|(cred_response, (pubkey, key_id))| {
                        // Convert the response into an `Mdoc`, verifying it against both the
                        // trust anchors and the `UnsignedMdoc` we received in the preview.
                        cred_response.into_mdoc::<K>(key_id, &pubkey, preview, trust_anchors)
                    })
                    .collect::<Result<_, _>>()?;

                // For each preview we have an `MdocCopies` instance.
                Ok(MdocCopies { cred_copies })
            })
            .collect::<Result<_, IssuanceSessionError>>()?;

        Ok(mdocs)
    }

    async fn reject_issuance(self) -> Result<(), IssuanceSessionError> {
        let url = Self::discover_batch_credential_endpoint(&self.message_client, &self.session_state.issuer_url)
            .await?
            .ok_or(IssuanceSessionError::NoBatchCredentialEndpoint)?;
        let (dpop_header, access_token_header) = self.session_state.auth_headers(url.clone(), Method::DELETE).await?;

        self.message_client
            .reject(&url, &dpop_header, &access_token_header)
            .await?;

        Ok(())
    }
}

impl CredentialResponse {
    /// Create an [`Mdoc`] out of the credential response. Also verifies the mdoc.
    fn into_mdoc<K: MdocEcdsaKey>(
        self,
        key_id: String,
        verifying_key: &VerifyingKey,
        preview: &AttestationPreview,
        trust_anchors: &[TrustAnchor<'_>],
    ) -> Result<Mdoc, IssuanceSessionError> {
        let issuer_signed = match self {
            CredentialResponse::MsoMdoc { credential } => credential.0,
        };

        if issuer_signed
            .public_key()
            .map_err(IssuanceSessionError::PublicKeyFromMdoc)?
            != *verifying_key
        {
            return Err(IssuanceSessionError::PublicKeyMismatch);
        }

        // Calculate the minimum of all the lengths of the random bytes
        // included in the attributes of `IssuerSigned`. If this value
        // is too low, we should not accept the attributes.
        if let Some(name_spaces) = issuer_signed.name_spaces.as_ref() {
            let min_random_length = name_spaces
                .values()
                .flat_map(|attributes| attributes.0.iter().map(|item| item.0.random.len()))
                .min();

            if let Some(min_random_length) = min_random_length {
                if min_random_length < ATTR_RANDOM_LENGTH {
                    return Err(IssuanceSessionError::AttributeRandomLength(
                        min_random_length,
                        ATTR_RANDOM_LENGTH,
                    ));
                }
            }
        }

        // The issuer certificate inside the mdoc has to equal the one that the issuer previously announced
        // in the attestation preview.
        let AttestationPreview::MsoMdoc { unsigned_mdoc, issuer } = preview;
        if issuer_signed.issuer_auth.signing_cert()? != *issuer {
            return Err(IssuanceSessionError::IssuerCertificateMismatch);
        }

        // Construct the new mdoc; this also verifies it against the trust anchors.
        let mdoc = Mdoc::new::<K>(key_id, issuer_signed, &TimeGenerator, trust_anchors)
            .map_err(IssuanceSessionError::MdocVerification)?;

        // Check that our mdoc contains exactly the attributes the issuer said it would have
        mdoc.compare_unsigned(unsigned_mdoc)
            .map_err(IssuanceSessionError::IssuedAttributesMismatch)?;

        Ok(mdoc)
    }
}

impl IssuanceState {
    async fn auth_headers(&self, url: Url, method: reqwest::Method) -> Result<(String, String), IssuanceSessionError> {
        let dpop_header = Dpop::new(
            &self.dpop_private_key,
            url,
            method,
            Some(&self.access_token),
            self.dpop_nonce.clone(),
        )
        .await?;

        let access_token_header = "DPoP ".to_string() + self.access_token.as_ref();

        Ok((dpop_header.into(), access_token_header))
    }
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;
    use nl_wallet_mdoc::{
        server_keys::KeyPair,
        software_key_factory::SoftwareKeyFactory,
        test::data,
        unsigned::UnsignedMdoc,
        utils::{
            issuer_auth::IssuerRegistration,
            serialization::{CborBase64, TaggedBytes},
        },
        Attributes, IssuerSigned,
    };
    use serde_bytes::ByteBuf;
    use wallet_common::keys::{software::SoftwareEcdsaKey, EcdsaKey};

    use super::*;

    async fn create_credential_response() -> (CredentialResponse, AttestationPreview, Certificate, VerifyingKey) {
        let ca = KeyPair::generate_issuer_mock_ca().unwrap();
        let issuance_key = ca.generate_issuer_mock(IssuerRegistration::new_mock().into()).unwrap();
        let key_factory = SoftwareKeyFactory::default();

        let unsigned_mdoc = UnsignedMdoc::from(data::pid_family_name().into_first().unwrap());
        let preview = AttestationPreview::MsoMdoc {
            unsigned_mdoc: unsigned_mdoc.clone(),
            issuer: issuance_key.certificate().clone(),
        };

        let mdoc_key = key_factory.generate_new().await.unwrap();
        let mdoc_public_key = mdoc_key.verifying_key().await.unwrap();
        let issuer_signed = IssuerSigned::sign(unsigned_mdoc, (&mdoc_public_key).try_into().unwrap(), &issuance_key)
            .await
            .unwrap();
        let credential_response = CredentialResponse::MsoMdoc {
            credential: issuer_signed.into(),
        };

        (credential_response, preview, ca.certificate().clone(), mdoc_public_key)
    }

    #[tokio::test]
    async fn test_credential_response_into_mdoc() {
        let (credential_response, preview, ca_cert, mdoc_public_key) = create_credential_response().await;

        let _ = credential_response
            .into_mdoc::<SoftwareEcdsaKey>(
                "key_id".to_string(),
                &mdoc_public_key,
                &preview,
                &[((&ca_cert).try_into().unwrap())],
            )
            .expect("should be able to convert CredentialResponse into Mdoc");
    }

    #[tokio::test]
    async fn test_credential_response_into_mdoc_public_key_mismatch_error() {
        let (credential_response, preview, ca_cert, _) = create_credential_response().await;

        // Converting a `CredentialResponse` into an `Mdoc` using a different mdoc
        // public key than the one contained within the response should fail.
        let other_public_key = *SigningKey::random(&mut OsRng).verifying_key();
        let error = credential_response
            .into_mdoc::<SoftwareEcdsaKey>(
                "key_id".to_string(),
                &other_public_key,
                &preview,
                &[((&ca_cert).try_into().unwrap())],
            )
            .expect_err("should not be able to convert CredentialResponse into Mdoc");

        assert_matches!(error, IssuanceSessionError::PublicKeyMismatch)
    }

    #[tokio::test]
    async fn test_credential_response_into_mdoc_attribute_random_length_error() {
        let (credential_response, preview, ca_cert, mdoc_public_key) = create_credential_response().await;

        // Converting a `CredentialResponse` into an `Mdoc` from a response
        // that contains insufficient random data should fail.
        let credential_response = match credential_response {
            CredentialResponse::MsoMdoc { mut credential } => {
                let CborBase64(ref mut credential_inner) = credential;
                let namespaces = credential_inner.name_spaces.as_mut().unwrap();
                let (_, Attributes(issuer_signed_items)) = namespaces.first_mut().unwrap();
                let TaggedBytes(first_item) = issuer_signed_items.first_mut().unwrap();

                first_item.random = ByteBuf::from(b"12345");

                CredentialResponse::MsoMdoc { credential }
            }
        };

        let error = credential_response
            .into_mdoc::<SoftwareEcdsaKey>(
                "key_id".to_string(),
                &mdoc_public_key,
                &preview,
                &[((&ca_cert).try_into().unwrap())],
            )
            .expect_err("should not be able to convert CredentialResponse into Mdoc");

        assert_matches!(
            error,
            IssuanceSessionError::AttributeRandomLength(5, ATTR_RANDOM_LENGTH)
        )
    }

    #[tokio::test]
    async fn test_credential_response_into_mdoc_issuer_certificate_mismatch_error() {
        let (credential_response, preview, ca_cert, mdoc_public_key) = create_credential_response().await;

        // Converting a `CredentialResponse` into an `Mdoc` using a different issuer
        // public key in the preview than is contained within the response should fail.
        let other_ca = KeyPair::generate_issuer_mock_ca().unwrap();
        let other_issuance_key = other_ca
            .generate_issuer_mock(IssuerRegistration::new_mock().into())
            .unwrap();
        let preview = match preview {
            AttestationPreview::MsoMdoc {
                unsigned_mdoc,
                issuer: _,
            } => AttestationPreview::MsoMdoc {
                unsigned_mdoc,
                issuer: other_issuance_key.certificate().clone(),
            },
        };

        let error = credential_response
            .into_mdoc::<SoftwareEcdsaKey>(
                "key_id".to_string(),
                &mdoc_public_key,
                &preview,
                &[((&ca_cert).try_into().unwrap())],
            )
            .expect_err("should not be able to convert CredentialResponse into Mdoc");

        assert_matches!(error, IssuanceSessionError::IssuerCertificateMismatch)
    }

    #[tokio::test]
    async fn test_credential_response_into_mdoc_mdoc_verification_error() {
        let (credential_response, preview, _, mdoc_public_key) = create_credential_response().await;

        // Converting a `CredentialResponse` into an `Mdoc` that is
        // validated against incorrect trust anchors should fail.
        let error = credential_response
            .into_mdoc::<SoftwareEcdsaKey>("key_id".to_string(), &mdoc_public_key, &preview, &[])
            .expect_err("should not be able to convert CredentialResponse into Mdoc");

        assert_matches!(error, IssuanceSessionError::MdocVerification(_))
    }

    #[tokio::test]
    async fn test_credential_response_into_mdoc_issued_attributes_mismatch_error() {
        let (credential_response, preview, ca_cert, mdoc_public_key) = create_credential_response().await;

        // Converting a `CredentialResponse` into an `Mdoc` with different attributes
        // in the preview than are contained within the response should fail.
        let preview = match preview {
            AttestationPreview::MsoMdoc {
                unsigned_mdoc: _,
                issuer,
            } => AttestationPreview::MsoMdoc {
                unsigned_mdoc: UnsignedMdoc::from(data::pid_full_name().into_first().unwrap()),
                issuer,
            },
        };

        let error = credential_response
            .into_mdoc::<SoftwareEcdsaKey>(
                "key_id".to_string(),
                &mdoc_public_key,
                &preview,
                &[((&ca_cert).try_into().unwrap())],
            )
            .expect_err("should not be able to convert CredentialResponse into Mdoc");

        assert_matches!(
            error,
            IssuanceSessionError::IssuedAttributesMismatch(IssuedAttributesMismatch { missing, unexpected })
                if missing.len() == 1 && unexpected.is_empty()
        )
    }
}
