//! RP software, for verifying mdoc disclosures, see [`DeviceResponse::verify()`].

use std::sync::Arc;

use base64::prelude::*;
use chrono::{DateTime, Utc};
use itertools::Itertools;
use josekit::{
    jwk::alg::ec::{EcCurve, EcKeyPair},
    JoseError,
};
use ring::hmac;
use serde::{Deserialize, Serialize};
use serde_with::{hex::Hex, serde_as};
use strum;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};
use url::Url;

use nl_wallet_mdoc::{
    holder::TrustAnchor,
    server_state::{
        Expirable, HasProgress, Progress, SessionState, SessionStore, SessionStoreError, SessionToken,
        CLEANUP_INTERVAL_SECONDS,
    },
    utils::x509::CertificateError,
    verifier::{
        DisclosedAttributes, ItemsRequests, ReturnUrlTemplate, SessionType, SessionTypeReturnUrl, UseCases,
        EPHEMERAL_ID_VALIDITY_SECONDS,
    },
};
use wallet_common::{
    config::wallet_config::BaseUrl,
    generator::Generator,
    jwt::{Jwt, JwtError},
    trust_anchor::OwnedTrustAnchor,
    utils::random_string,
};

use crate::{
    authorization::AuthorizationErrorCode,
    jwt,
    openid4vp::{
        AuthRequestError, AuthResponseError, VpAuthorizationErrorCode, VpAuthorizationRequest, VpAuthorizationResponse,
        VpRequestUriObject, VpResponse,
    },
    ErrorResponse,
};

/// Errors that can occur during processing of any of the endpoints.
#[derive(thiserror::Error, Debug)]
pub enum SessionError {
    #[error("session not in expected state")]
    UnexpectedState,
    #[error("unknown session: {0}")]
    UnknownSession(SessionToken),
    #[error("error with sessionstore: {0}")]
    SessionStore(SessionStoreError),
}

/// Errors returned by endpoints used by the RP.
#[derive(thiserror::Error, Debug)]
pub enum VerificationError {
    #[error("session error: {0}")]
    Session(#[from] SessionError),

    // RP errors
    #[error("session is done")]
    SessionIsDone,
    #[error("unknown use case: {0}")]
    UnknownUseCase(String),
    #[error("presence or absence of return url template does not match configuration for the required use case")]
    ReturnUrlConfigurationMismatch,
    #[error("no ItemsRequest: can't request a disclosure of 0 attributes")]
    NoItemsRequests,
    #[error("disclosed attributes requested for disclosure session with status other than 'Done'")]
    SessionNotDone,
    #[error("redirect URI nonce '{0}' does not match expected")]
    RedirectUriMismatch(String),
    #[error("missing redirect URI")]
    RedirectUriMissing,
    #[error("missing DNS SAN from RP certificate")]
    MissingSAN,
    #[error("RP certificate error: {0}")]
    Certificate(#[from] CertificateError),

    // status endpoint error
    #[error("URL encoding error: {0}")]
    UrlEncoding(#[from] serde_urlencoded::ser::Error),
}

/// Errors returned by the endpoint that returns the Authorization Request.
#[derive(thiserror::Error, Debug)]
pub enum GetAuthRequestError {
    #[error("session error: {0}")]
    Session(#[from] SessionError),
    #[error("the ephemeral ID {} is invalid", hex::encode(.0))]
    InvalidEphemeralId(Vec<u8>),
    #[error("the ephemeral ID {} has expired", hex::encode(.0))]
    ExpiredEphemeralId(Vec<u8>),
    #[error("error creating ephemeral encryption keypair: {0}")]
    EncryptionKey(#[from] JoseError),
    #[error("error creating Authorization Request: {0}")]
    AuthRequest(#[from] AuthRequestError),
    #[error("error signing Authorization Request JWE: {0}")]
    Jwt(#[from] JwtError),
}

/// Errors returned by the endpoint to which the user posts the Authorization Response.
#[derive(thiserror::Error, Debug)]
pub enum PostAuthResponseError {
    #[error("session error: {0}")]
    Session(#[from] SessionError),
    #[error("error decrypting or verifying Authorization Response JWE: {0}")]
    AuthResponse(#[from] AuthResponseError),
    #[error("user aborted with error: {0:?}")]
    UserError(ErrorResponse<VpAuthorizationErrorCode>),
}

/// A disclosure session. `S` must implement [`DisclosureState`] and is the state that the session is in.
/// The session progresses through the possible states using a state engine that uses the typestate pattern:
/// for each state `S`, `Session<S>` has its own state transition method that consume the previous state.
#[derive(Debug)]
struct Session<S: DisclosureState> {
    state: SessionState<S>,
}

/// State for a session that has just been created.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Created {
    items_requests: ItemsRequests,
    usecase_id: String,
    client_id: String,
    redirect_uri_template: Option<ReturnUrlTemplate>,
}

/// State for a session that is waiting for the user's disclosure, i.e., the device has contacted us at the session URL.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WaitingForResponse {
    auth_request: VpAuthorizationRequest,
    encryption_key: EncryptionPrivateKey,
    redirect_uri: Option<RedirectUri>,
}

/// State for a session that has ended (for any reason).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Done {
    session_result: SessionResult,
}

/// The outcome of a session: the disclosed attributes if they have been sucessfully received and verified.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "UPPERCASE", tag = "status")]
pub enum SessionResult {
    Done {
        disclosed_attributes: DisclosedAttributes,
        redirect_uri_nonce: Option<String>,
    },
    Failed {
        error: String,
    },
    Cancelled,
    Expired,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct RedirectUri {
    template: ReturnUrlTemplate,
    nonce: String,
}

impl RedirectUri {
    fn into_url(self, session_token: &SessionToken) -> Url {
        let mut url = self.template.into_url(session_token);
        url.query_pairs_mut().append_pair("nonce", &self.nonce);
        url
    }
}

/// Wrapper for [`EcKeyPair`] that can be serialized.
#[derive(Debug, Clone)]
struct EncryptionPrivateKey(EcKeyPair);

impl Serialize for EncryptionPrivateKey {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        BASE64_URL_SAFE_NO_PAD
            .encode(self.0.to_der_private_key())
            .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EncryptionPrivateKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(EncryptionPrivateKey(
            EcKeyPair::from_der(
                BASE64_URL_SAFE_NO_PAD
                    .decode(String::deserialize(deserializer)?)
                    .map_err(serde::de::Error::custom)?,
                None,
            )
            .map_err(serde::de::Error::custom)?,
        ))
    }
}

/// Sent by the wallet to the `response_uri`: either an Authorization Response JWE or an error, which either indicates
/// that they refuse disclosure, or is an actual error that the wallet encountered during the session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WalletAuthResponse {
    Response(String),
    Error(ErrorResponse<VpAuthorizationErrorCode>),
}

/// Disclosure session states for use as `T` in `Session<T>`.
pub trait DisclosureState {}

impl DisclosureState for Created {}
impl DisclosureState for WaitingForResponse {}
impl DisclosureState for Done {}

/// Disclosure-specific session data, of any state, for storing in a session store.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DisclosureData {
    Created(Created),
    WaitingForResponse(WaitingForResponse),
    Done(Done),
}

impl HasProgress for DisclosureData {
    fn progress(&self) -> Progress {
        match self {
            Self::Created(_) | Self::WaitingForResponse(_) => Progress::Active,
            Self::Done(done) => Progress::Finished {
                has_succeeded: matches!(done.session_result, SessionResult::Done { .. }),
            },
        }
    }
}

impl Expirable for DisclosureData {
    fn is_expired(&self) -> bool {
        matches!(
            self,
            Self::Done(Done {
                session_result: SessionResult::Expired
            })
        )
    }

    fn expire(&mut self) {
        *self = Self::Done(Done {
            session_result: SessionResult::Expired,
        })
    }
}

// From/TryFrom converters for the various state structs to the `DisclosureData` enum

impl From<Session<Created>> for SessionState<DisclosureData> {
    fn from(value: Session<Created>) -> Self {
        SessionState {
            data: DisclosureData::Created(value.state.data),
            token: value.state.token,
            last_active: value.state.last_active,
        }
    }
}

impl TryFrom<SessionState<DisclosureData>> for Session<Created> {
    type Error = SessionError;

    fn try_from(value: SessionState<DisclosureData>) -> Result<Self, Self::Error> {
        let DisclosureData::Created(session_data) = value.data else {
            return Err(SessionError::UnexpectedState);
        };
        Ok(Session::<Created> {
            state: SessionState {
                data: session_data,
                token: value.token,
                last_active: value.last_active,
            },
        })
    }
}

impl From<Session<WaitingForResponse>> for SessionState<DisclosureData> {
    fn from(value: Session<WaitingForResponse>) -> Self {
        SessionState {
            data: DisclosureData::WaitingForResponse(value.state.data),
            token: value.state.token,
            last_active: value.state.last_active,
        }
    }
}

impl TryFrom<SessionState<DisclosureData>> for Session<WaitingForResponse> {
    type Error = SessionError;

    fn try_from(value: SessionState<DisclosureData>) -> Result<Self, Self::Error> {
        let DisclosureData::WaitingForResponse(session_data) = value.data else {
            return Err(SessionError::UnexpectedState);
        };
        Ok(Session::<WaitingForResponse> {
            state: SessionState {
                data: session_data,
                token: value.token,
                last_active: value.last_active,
            },
        })
    }
}

impl From<Session<Done>> for SessionState<DisclosureData> {
    fn from(value: Session<Done>) -> Self {
        SessionState {
            data: DisclosureData::Done(value.state.data),
            token: value.state.token,
            last_active: value.state.last_active,
        }
    }
}

/// Session status for the frontend.
#[derive(Debug, Deserialize, Serialize, strum::Display)]
#[serde(rename_all = "UPPERCASE", tag = "status")]
pub enum StatusResponse {
    Created { ul: BaseUrl },
    WaitingForResponse,
    Done,
    Failed,
    Cancelled,
    Expired,
}

pub struct Verifier<S> {
    use_cases: UseCases,
    sessions: Arc<S>,
    cleanup_task: JoinHandle<()>,
    trust_anchors: Vec<OwnedTrustAnchor>,
    ephemeral_id_secret: hmac::Key,
}

impl<S> Drop for Verifier<S> {
    fn drop(&mut self) {
        // Stop the task at the next .await
        self.cleanup_task.abort();
    }
}

impl<S> Verifier<S>
where
    S: SessionStore<DisclosureData>,
{
    /// Create a new [`Verifier`].
    ///
    /// - `use_cases` contains configuration per use case, including a certificate
    ///    and corresponding private key for use in RP authentication.
    /// - `sessions` will contain all sessions.
    /// - `trust_anchors` contains self-signed X509 CA certificates acting as trust anchor for the mdoc verification:
    ///   the mdoc verification function [`Document::verify()`] returns true if the mdoc verifies against one of these CAs.
    /// - `ephemeral_id_secret` is used as a HMAC secret to create ephemeral session IDs.
    pub fn new(
        use_cases: UseCases,
        sessions: S,
        trust_anchors: Vec<OwnedTrustAnchor>,
        ephemeral_id_secret: hmac::Key,
    ) -> Self
    where
        S: Send + Sync + 'static,
    {
        let sessions = Arc::new(sessions);
        Self {
            use_cases,
            cleanup_task: sessions.clone().start_cleanup_task(CLEANUP_INTERVAL_SECONDS),
            sessions,
            trust_anchors,
            ephemeral_id_secret,
        }
    }

    /// Start a new disclosure session. Returns a [`SessionToken`] that can be used to retrieve the
    /// session state.
    ///
    /// - `items_requests` contains the attributes to be requested.
    /// - `usecase_id` should point to an existing item in the `certificates` parameter.
    /// - `return_url_template` is the return URL the user should be returned to, if present.
    pub async fn new_session(
        &self,
        items_requests: ItemsRequests,
        usecase_id: String,
        return_url_template: Option<ReturnUrlTemplate>,
    ) -> Result<SessionToken, VerificationError> {
        info!("create verifier session: {usecase_id}");

        if items_requests.0.is_empty() {
            return Err(VerificationError::NoItemsRequests);
        }

        let use_case = self
            .use_cases
            .as_ref()
            .get(&usecase_id)
            .ok_or_else(|| VerificationError::UnknownUseCase(usecase_id.clone()))?;

        // Check if we should or should not have received a return URL
        // template, based on the configuration for the use case.
        if match use_case.session_type_return_url {
            SessionTypeReturnUrl::Neither => return_url_template.is_some(),
            SessionTypeReturnUrl::SameDevice | SessionTypeReturnUrl::Both => return_url_template.is_none(),
        } {
            return Err(VerificationError::ReturnUrlConfigurationMismatch);
        }

        let client_id = use_case
            .key_pair
            .certificate()
            .san_dns_name()?
            .ok_or(VerificationError::MissingSAN)?;

        let (session_token, session_state) =
            Session::<Created>::new(items_requests, usecase_id, client_id, return_url_template)?;

        self.sessions
            .write(session_state.into(), true)
            .await
            .map_err(SessionError::SessionStore)?;

        info!("Session({session_token}): session created");
        Ok(session_token)
    }

    fn verify_ephemeral_id(
        &self,
        session_token: &SessionToken,
        url_params: &VerifierUrlParameters,
    ) -> Result<(), GetAuthRequestError> {
        if Utc::now() - EPHEMERAL_ID_VALIDITY_SECONDS > url_params.time {
            return Err(GetAuthRequestError::ExpiredEphemeralId(url_params.ephemeral_id.clone()));
        }
        hmac::verify(
            &self.ephemeral_id_secret,
            &Self::format_ephemeral_id_payload(session_token, &url_params.time),
            &url_params.ephemeral_id,
        )
        .map_err(|_| GetAuthRequestError::InvalidEphemeralId(url_params.ephemeral_id.clone()))?;

        Ok(())
    }

    pub async fn process_get_request(
        &self,
        session_token: &SessionToken,
        verifier_base_url: &BaseUrl,
        url_params: VerifierUrlParameters,
    ) -> Result<Jwt<VpAuthorizationRequest>, GetAuthRequestError> {
        let session: Session<Created> = self
            .sessions
            .get(session_token)
            .await
            .map_err(SessionError::SessionStore)?
            .ok_or_else(|| SessionError::UnknownSession(session_token.clone()))?
            .try_into()?;

        info!("Session({session_token}): get request");

        // Verify the ephemeral ID here as opposed to inside `session.process_get_request()`, so that if the
        // ephemeral ID is too old e.g. because the user's internet connection was very slow, then we don't fail the
        // session. This means that the QR code/UL stays on the website so that the user can try again.
        self.verify_ephemeral_id(session_token, &url_params)?;

        let (result, next) = match session
            .process_get_request(verifier_base_url, url_params.session_type, &self.use_cases)
            .await
        {
            Ok((jws, next)) => (Ok(jws), next.into()),
            Err((err, next)) => (Err(err), next.into()),
        };

        self.sessions
            .write(next, false)
            .await
            .map_err(SessionError::SessionStore)?;

        result
    }

    pub async fn process_authorization_response(
        &self,
        session_token: &SessionToken,
        wallet_response: WalletAuthResponse,
        time: &impl Generator<DateTime<Utc>>,
    ) -> Result<VpResponse, PostAuthResponseError> {
        let session: Session<WaitingForResponse> = self
            .sessions
            .get(session_token)
            .await
            .map_err(SessionError::SessionStore)?
            .ok_or_else(|| SessionError::UnknownSession(session_token.clone()))?
            .try_into()?;

        let (result, next) = session.process_authorization_response(
            session_token,
            wallet_response,
            time,
            self.trust_anchors
                .iter()
                .map(Into::<TrustAnchor<'_>>::into)
                .collect_vec()
                .as_slice(),
        );

        self.sessions
            .write(next.into(), false)
            .await
            .map_err(SessionError::SessionStore)?;

        result
    }

    pub async fn status_response(
        &self,
        session_token: &SessionToken,
        ul_base: &BaseUrl,
        verifier_base_url: &BaseUrl,
        session_type: SessionType,
    ) -> Result<StatusResponse, VerificationError> {
        let response = match self
            .sessions
            .get(session_token)
            .await
            .map_err(SessionError::SessionStore)?
            .ok_or_else(|| SessionError::UnknownSession(session_token.clone()))?
            .data
        {
            DisclosureData::Created(Created { client_id, .. }) => {
                let time = Utc::now();
                let ul = Self::format_ul(
                    ul_base,
                    verifier_base_url,
                    session_token,
                    time,
                    self.generate_ephemeral_id(session_token, &time),
                    session_type,
                    client_id,
                )?;
                StatusResponse::Created { ul }
            }
            DisclosureData::WaitingForResponse(_) => StatusResponse::WaitingForResponse,
            DisclosureData::Done(Done {
                session_result: SessionResult::Done { .. },
            }) => StatusResponse::Done,
            DisclosureData::Done(Done {
                session_result: SessionResult::Failed { .. },
            }) => StatusResponse::Failed,
            DisclosureData::Done(Done {
                session_result: SessionResult::Cancelled { .. },
            }) => StatusResponse::Cancelled,
            DisclosureData::Done(Done {
                session_result: SessionResult::Expired { .. },
            }) => StatusResponse::Expired,
        };

        Ok(response)
    }

    /// Returns the disclosed attributes for a session with status `Done` and an error otherwise
    pub async fn disclosed_attributes(
        &self,
        session_token: &SessionToken,
        redirect_uri_nonce: Option<String>,
    ) -> Result<DisclosedAttributes, VerificationError> {
        let disclosure_data = self
            .sessions
            .get(session_token)
            .await
            .map_err(SessionError::SessionStore)?
            .ok_or_else(|| SessionError::UnknownSession(session_token.clone()))?
            .data;

        match disclosure_data {
            DisclosureData::Done(Done {
                session_result:
                    SessionResult::Done {
                        redirect_uri_nonce: expected_nonce,
                        disclosed_attributes,
                    },
            }) => match (redirect_uri_nonce, expected_nonce) {
                (_, None) => Ok(disclosed_attributes),
                (None, Some(_)) => Err(VerificationError::RedirectUriMissing),
                (Some(received), Some(expected)) if received == expected => Ok(disclosed_attributes),
                (Some(received), Some(_)) => Err(VerificationError::RedirectUriMismatch(received)),
            },
            _ => Err(VerificationError::SessionNotDone),
        }
    }
}

impl<S> Verifier<S> {
    fn generate_ephemeral_id(&self, session_token: &SessionToken, time: &DateTime<Utc>) -> Vec<u8> {
        let ephemeral_id = hmac::sign(
            &self.ephemeral_id_secret,
            &Self::format_ephemeral_id_payload(session_token, time),
        )
        .as_ref()
        .to_vec();
        ephemeral_id
    }

    fn format_ul(
        base_ul: &BaseUrl,
        verifier_base_url: &BaseUrl,
        session_token: &SessionToken,
        time: DateTime<Utc>,
        ephemeral_id: Vec<u8>,
        session_type: SessionType,
        client_id: String,
    ) -> Result<BaseUrl, VerificationError> {
        let mut request_uri = verifier_base_url
            .join_base_url("request_uri")
            .join_base_url(session_token.as_ref())
            .into_inner();
        request_uri.set_query(Some(&serde_urlencoded::to_string(VerifierUrlParameters {
            time,
            ephemeral_id,
            session_type,
        })?));

        let mut ul = base_ul.clone().into_inner();
        ul.set_query(Some(&serde_urlencoded::to_string(VpRequestUriObject {
            request_uri: request_uri.try_into().unwrap(), // safe because we constructed request_uri from a BaseUrl
            client_id,
        })?));

        Ok(ul.try_into().unwrap()) // safe because we constructed request_uri from a BaseUrl
    }

    // formats the payload to hash to the ephemeral ID in a consistent way
    fn format_ephemeral_id_payload(session_token: &SessionToken, time: &DateTime<Utc>) -> Vec<u8> {
        // default (de)serialization of DateTime is the RFC 3339 format
        format!("{}|{}", session_token, time.to_rfc3339()).into()
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifierUrlParameters {
    pub session_type: SessionType,
    #[serde_as(as = "Hex")]
    pub ephemeral_id: Vec<u8>,
    // default (de)serialization of DateTime is the RFC 3339 format
    pub time: DateTime<Utc>,
}

// Implementation of the typestate state engine follows.

// Transitioning functions and helpers valid for any state
impl<T: DisclosureState> Session<T> {
    fn transition_fail(self, error: &impl ToString) -> Session<Done> {
        self.transition(Done {
            session_result: SessionResult::Failed {
                error: error.to_string(),
            },
        })
    }

    /// Transition `self` to a new state, consuming the old state, also updating the `last_active` timestamp.
    fn transition<NewT: DisclosureState>(self, new_state: NewT) -> Session<NewT> {
        Session {
            state: SessionState::new(self.state.token, new_state),
        }
    }

    fn state(&self) -> &T {
        &self.state.data
    }
}

impl Session<Created> {
    /// Create a new disclosure session.
    fn new(
        items_requests: ItemsRequests,
        usecase_id: String,
        client_id: String,
        return_url_template: Option<ReturnUrlTemplate>,
    ) -> Result<(SessionToken, Session<Created>), VerificationError> {
        let session_token = SessionToken::new_random();
        let session = Session::<Created> {
            state: SessionState::new(
                session_token.clone(),
                Created {
                    items_requests,
                    usecase_id,
                    client_id,
                    redirect_uri_template: return_url_template,
                },
            ),
        };

        Ok((session_token, session))
    }

    /// Process the device's request for the Authorization Request,
    /// returning a response to answer the device with and the next session state.
    async fn process_get_request(
        self,
        verifier_base_url: &BaseUrl,
        session_type: SessionType,
        use_cases: &UseCases,
    ) -> Result<(Jwt<VpAuthorizationRequest>, Session<WaitingForResponse>), (GetAuthRequestError, Session<Done>)> {
        info!("Session({}): process get request", self.state.token);

        let (response, next) = match self
            .process_get_request_inner(verifier_base_url, session_type, use_cases)
            .await
        {
            Ok((jws, auth_request, redirect_uri, enc_keypair)) => {
                let next = WaitingForResponse {
                    auth_request,
                    encryption_key: EncryptionPrivateKey(enc_keypair),
                    redirect_uri,
                };
                let next = self.transition(next);
                Ok((jws, next))
            }
            Err(err) => {
                warn!(
                    "Session({}): process get request failed, returning error",
                    self.state.token
                );
                let next = self.transition_fail(&err);
                Err((err, next))
            }
        }?;

        Ok((response, next))
    }

    // Helper function that returns ordinary errors instead of `Session<...>`
    async fn process_get_request_inner(
        &self,
        verifier_base_url: &BaseUrl,
        session_type: SessionType,
        use_cases: &UseCases,
    ) -> Result<
        (
            Jwt<VpAuthorizationRequest>,
            VpAuthorizationRequest,
            Option<RedirectUri>,
            EcKeyPair,
        ),
        GetAuthRequestError,
    > {
        let usecase = &use_cases
            .as_ref()
            .get(&self.state().usecase_id)
            .expect("usecase_id should always refers to existing use case");

        // Determine if we should include a redirect URI, based on the use case configuration and session type.
        let redirect_uri = match (
            usecase.session_type_return_url,
            session_type,
            self.state().redirect_uri_template.clone(),
        ) {
            (SessionTypeReturnUrl::Both, _, Some(uri_template))
            | (SessionTypeReturnUrl::SameDevice, SessionType::SameDevice, Some(uri_template)) => Some(RedirectUri {
                template: uri_template.clone(),
                nonce: random_string(32),
            }),
            (SessionTypeReturnUrl::Neither, _, _) | (SessionTypeReturnUrl::SameDevice, SessionType::CrossDevice, _) => {
                None
            }
            _ => panic!("return URL configuration mismatch"), // We checked for this case when the session was created
        };

        // Construct the Authorization Request.
        let nonce = random_string(32);
        let response_uri = verifier_base_url
            .join_base_url("response_uri")
            .join_base_url(self.state.token.as_ref());
        let encryption_keypair = EcKeyPair::generate(EcCurve::P256)?;
        let auth_request = VpAuthorizationRequest::new(
            &self.state.data.items_requests,
            usecase.key_pair.certificate(),
            nonce.clone(),
            encryption_keypair.to_jwk_public_key().try_into().unwrap(), // safe because we just constructed this key
            response_uri,
        )?;

        let jws = jwt::sign_with_certificate(&auth_request, &usecase.key_pair).await?;

        Ok((jws, auth_request, redirect_uri, encryption_keypair))
    }
}

impl Session<WaitingForResponse> {
    /// Process the user's encrypted `VpAuthorizationResponse`, i.e. its disclosure,
    /// returning a response to answer the device with and the next session state.
    ///
    /// Unlike many similar method, this method does not have an `_inner()` version that returns `Result<_,_>`
    /// because it differs from similar methods in the following aspect: in some cases (to wit, if the user
    /// sent an error instead of a disclosure) then we should respond with HTTP 200 to the user (mandated by
    /// the OpenID4VP spec), while we fail our session. This does not neatly fit in the `_inner()` method pattern.
    fn process_authorization_response(
        self,
        session_token: &SessionToken,
        wallet_response: WalletAuthResponse,
        time: &impl Generator<DateTime<Utc>>,
        trust_anchors: &[TrustAnchor],
    ) -> (Result<VpResponse, PostAuthResponseError>, Session<Done>) {
        debug!("Session({}): process response", self.state.token);

        let jwe = match wallet_response {
            WalletAuthResponse::Response(jwe) => jwe,
            WalletAuthResponse::Error(err) => {
                // Check if the error code indicates that the user refused to disclose.
                let user_refused = matches!(
                    err.error,
                    VpAuthorizationErrorCode::AuthorizationError(AuthorizationErrorCode::AccessDenied)
                );

                let response = self.ok_response(session_token);
                let next = if user_refused {
                    self.transition_abort()
                } else {
                    // If the user sent any other error, fail the session.
                    self.transition_fail(&PostAuthResponseError::UserError(err))
                };
                // Return a non-error response to the wallet (including the redirect URI) to indicate
                // we successfully processed its error response.
                return (Ok(response), next);
            }
        };

        debug!(
            "Session({}): process response: decrypting and deserializing Authorization Response JWE",
            self.state.token
        );
        let (result, next) = match VpAuthorizationResponse::decrypt_and_verify(
            jwe,
            &self.state().encryption_key.0,
            &self.state().auth_request,
            time,
            trust_anchors,
        ) {
            Ok(disclosed) => {
                let redirect_uri_nonce = self.state().redirect_uri.as_ref().map(|r| r.nonce.clone());
                let response = self.ok_response(session_token);
                let next = self.transition_finish(disclosed, redirect_uri_nonce);
                (Ok(response), next)
            }
            Err(err) => {
                let next = self.transition_fail(&err);
                (Err(err.into()), next)
            }
        };

        (result, next)
    }

    fn ok_response(&self, session_token: &SessionToken) -> VpResponse {
        VpResponse {
            redirect_uri: self
                .state()
                .redirect_uri
                .as_ref()
                .map(|u| u.clone().into_url(session_token).try_into().unwrap()),
        }
    }

    fn transition_finish(self, disclosed_attributes: DisclosedAttributes, nonce: Option<String>) -> Session<Done> {
        self.transition(Done {
            session_result: SessionResult::Done {
                disclosed_attributes,
                redirect_uri_nonce: nonce,
            },
        })
    }

    fn transition_abort(self) -> Session<Done> {
        self.transition(Done {
            session_result: SessionResult::Cancelled,
        })
    }
}
