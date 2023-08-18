use std::marker::PhantomData;

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use url::Url;
use wallet_common::utils;

use crate::utils::url_find_first_query_value;

use super::{
    openid_client::{OpenIdClient, RemoteOpenIdClient},
    pkce::{PkceGenerator, PkceSource},
    DigidClient, DigidClientError,
};

const PARAM_ERROR: &str = "error";
const PARAM_ERROR_DESCRIPTION: &str = "error_description";
const PARAM_STATE: &str = "state";
const PARAM_CODE: &str = "code";

#[derive(Debug)]
pub struct RemoteDigidClient<C = RemoteOpenIdClient, P = PkceGenerator> {
    // Only one session may be active at a time. A potential improvement would be
    // to support multiple sessions and to persist these sessions, so that they may
    // be resumed after app termination.
    session_state: Option<DigidSessionState<C>>,
    _pkce_source: PhantomData<P>,
}

#[derive(Debug)]
struct DigidSessionState<C> {
    // The discovered OpenID client.
    openid_client: C,
    /// CSRF token (stored in state parameter).
    csrf_token: String,
    /// The generated nonce that was used.
    nonce: String,
    /// The PKCE verifier used.
    pkce_verifier: String,
}

impl<C, P> RemoteDigidClient<C, P> {
    fn new() -> Self {
        RemoteDigidClient {
            session_state: None,
            _pkce_source: PhantomData,
        }
    }
}

impl<C, P> RemoteDigidClient<C, P>
where
    C: OpenIdClient,
{
    // Helper static method for checking if redirect URI is accepted.
    fn openid_client_accepts_redirect_uri(openid_client: &C, redirect_uri: &Url) -> bool {
        redirect_uri.as_str().starts_with(openid_client.redirect_uri().as_str())
    }
}

impl<C, P> Default for RemoteDigidClient<C, P> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<C, P> DigidClient for RemoteDigidClient<C, P>
where
    P: PkceSource + Send + Sync,
    C: OpenIdClient + Send + Sync,
{
    async fn start_session(
        &mut self,
        issuer_url: &Url,
        client_id: &str,
        redirect_uri: &Url,
    ) -> Result<Url, DigidClientError> {
        // TODO: This performs discovery every time a session is started and an authentication URL
        //       is generated. An improvement would be to cache the OpenIdClient and only perform
        //       discovery again when the configuration parameters change.

        // Perform OpenID discovery at the issuer.
        let openid_client = C::discover(issuer_url.clone(), client_id.to_string(), redirect_uri.clone()).await?;

        // Generate a random PKCE verifier, CSRF token and nonce.
        let (pkce_verifier, pkce_challenge) = P::generate_verifier_and_challenge();
        let csrf_token = URL_SAFE_NO_PAD.encode(utils::random_bytes(16));
        let nonce = URL_SAFE_NO_PAD.encode(utils::random_bytes(16));

        let url = openid_client.auth_url(&csrf_token, &nonce, &pkce_challenge);

        // Store the client and generated tokens as session state for when the redirect URI returns.
        self.session_state.replace(DigidSessionState {
            openid_client,
            pkce_verifier,
            csrf_token,
            nonce,
        });

        Ok(url)
    }

    fn accepts_redirect_uri(&self, redirect_uri: &Url) -> bool {
        self.session_state
            .as_ref()
            .map(|state| Self::openid_client_accepts_redirect_uri(&state.openid_client, redirect_uri))
            .unwrap_or_default()
    }

    async fn get_access_token(&mut self, received_redirect_uri: &Url) -> Result<String, DigidClientError> {
        // Get the session state, return an error if we have none.
        let DigidSessionState {
            openid_client,
            pkce_verifier,
            csrf_token,
            nonce,
        } = self.session_state.as_ref().ok_or(DigidClientError::NoSession)?;

        // Check if the redirect URL received actually belongs to us.
        if !Self::openid_client_accepts_redirect_uri(openid_client, received_redirect_uri) {
            return Err(DigidClientError::RedirectUriMismatch);
        }

        // Check if the `error` query parameter is populated, if so create an
        // error from it and a potential `error_description` query parameter.
        let error = url_find_first_query_value(received_redirect_uri, PARAM_ERROR);
        if let Some(error) = error {
            let error_description = url_find_first_query_value(received_redirect_uri, PARAM_ERROR_DESCRIPTION);

            return Err(DigidClientError::RedirectUriError {
                error: error.into_owned(),
                error_description: error_description.map(|d| d.into_owned()),
            });
        }

        // Verify that the state query parameter matches the csrf_token.
        let state = url_find_first_query_value(received_redirect_uri, PARAM_STATE)
            .ok_or(DigidClientError::StateTokenMismatch)?;

        if state != *csrf_token {
            return Err(DigidClientError::StateTokenMismatch);
        }

        // Parse the authorization code from the redirect URL.
        let authorization_code =
            url_find_first_query_value(received_redirect_uri, PARAM_CODE).ok_or(DigidClientError::NoAuthCode)?;

        // Use the authorization code and the PKCE verifier to request the
        // access token and verify the result.
        let access_token = openid_client
            .authenticate(&authorization_code, nonce, pkce_verifier)
            .await?;

        // If everything succeeded, remove the session state.
        self.session_state.take();

        Ok(access_token)
    }
}

#[cfg(test)]
mod tests {
    use mockall::predicate::*;
    use once_cell::sync::Lazy;
    use tokio::sync::oneshot;

    use crate::digid::{openid_client::MockOpenIdClient, pkce::MockPkceSource};

    use super::*;

    #[tokio::test]
    async fn test_digid_client() {
        // Set up some constants that are returned by our mocks.
        static ISSUER_URL: Lazy<Url> = Lazy::new(|| Url::parse("http://example.com").unwrap());
        const CLIENT_ID: &str = "client-1";
        static REDIRECT_URI: Lazy<Url> = Lazy::new(|| Url::parse("redirect://here").unwrap());

        const PKCE_VERIFIER: &str = "a_pkce_verifier";
        const PKCE_CHALLENGE: &str = "a_pkce_challenge";
        const AUTH_URL: &str = "http://example.com/auth";
        const AUTH_CODE: &str = "the_authentication_code";
        const ACCESS_CODE: &str = "the_access_code";

        // Create a client with mock generics, as created by `mockall`.
        let mut client = RemoteDigidClient::<MockOpenIdClient, MockPkceSource>::default();

        // There should be no session state present at this point.
        assert!(client.session_state.is_none());

        // Also, we should not be accepting a valid redirect URIs.
        assert!(!client.accepts_redirect_uri(Lazy::force(&REDIRECT_URI)));

        // Setup a channel so that we can intercept the generated CSRF token and nonce,
        // which we will do when setting up the mock with closures.
        // NOTE: A potential improvement to this would be to place the `utils::random_bytes()`
        //       function behind and interface as well and mock that.
        let (tokens_tx, mut tokens_rx) = oneshot::channel::<(String, String)>();

        // Now prepare the our mock dependencies for us to call `DigidClient.start_session()`.
        // This means:
        // 1. Set up `OpenIdClient::discover_context()` to return a new mock.
        let discover_context = MockOpenIdClient::discover_context();
        discover_context
            .expect()
            .with(
                eq(Lazy::force(&ISSUER_URL)),
                eq(CLIENT_ID.to_string()),
                eq(Lazy::force(&REDIRECT_URI)),
            )
            .return_once(move |_, _, _| {
                let mut openid_client = MockOpenIdClient::new();

                // 2. Have `OpenIdClient.auth_url` return our authentication URL, while saving
                //    the generated CSRF token and nonce for later (send throught the channel).
                openid_client
                    .expect_auth_url()
                    .with(always(), always(), eq(PKCE_CHALLENGE))
                    .return_once(move |csrf_token, nonce, _| {
                        _ = tokens_tx.send((csrf_token.to_string(), nonce.to_string()));

                        Url::parse(AUTH_URL).unwrap()
                    });

                Ok(openid_client)
            });

        // 3. Set up `PkceSource::generate_verifier_and_challenge()` to return our
        //    static PKCE strings.
        let pkce_generate_context = MockPkceSource::generate_verifier_and_challenge_context();
        pkce_generate_context
            .expect()
            .return_const((PKCE_VERIFIER.to_string(), PKCE_CHALLENGE.to_string()));

        // Now we are ready to call `DigidClient.start_session()`, which should succeed.
        let url = client
            .start_session(Lazy::force(&ISSUER_URL), CLIENT_ID, Lazy::force(&REDIRECT_URI))
            .await
            .expect("Could not start DigiD session");

        // Check the return value.
        assert_eq!(url.as_str(), AUTH_URL);

        // Receive the generated tokens through the channel.
        let (generated_csrf_token, generated_nonce) = tokens_rx
            .try_recv()
            .expect("Generated tokens not set after session start");

        // Check the internal state of DigidClient.
        assert!(matches!(
            client.session_state,
            Some(DigidSessionState {
                openid_client: _,
                ref csrf_token,
                ref nonce,
                ref pkce_verifier,
            }) if csrf_token == &generated_csrf_token && nonce == &generated_nonce && pkce_verifier == PKCE_VERIFIER
        ));

        // Finally, make sure the mock methods were actually called.
        discover_context.checkpoint();
        pkce_generate_context.checkpoint();
        client.session_state.as_mut().unwrap().openid_client.checkpoint();

        // From this point on, `OpenIdClient::redirect_uri()` will be called
        // several times, so make sure it is returned.
        client
            .session_state
            .as_mut()
            .unwrap()
            .openid_client
            .expect_redirect_uri()
            .return_const(REDIRECT_URI.clone());

        // Now that there is an active session, a valid redirect URI should be accepted...
        assert!(client.accepts_redirect_uri(Lazy::force(&REDIRECT_URI)));

        // ...but an invalid one should not.
        assert!(!client.accepts_redirect_uri(&Url::parse("http://not-the-redirect-uri.com").unwrap()));

        // Next we test the `DigidClient.get_access_token()` method. We start
        // by going through some error cases.
        //
        // First, we test the error when provding a redirect URI that does not
        // match the one configured for the client.

        assert!(matches!(
            client
                .get_access_token(&Url::parse("http://not-the-redirect-uri.com").unwrap())
                .await
                .unwrap_err(),
            DigidClientError::RedirectUriMismatch
        ));

        // Test for redirect URIs that contain a `error` and an optional
        // `error_description` parameter.

        let error_redirect_uri = {
            let mut redirect_uri = REDIRECT_URI.clone();

            redirect_uri
                .query_pairs_mut()
                .append_pair(PARAM_ERROR, "error_type")
                .append_pair(PARAM_ERROR_DESCRIPTION, "this is the error description");

            redirect_uri
        };

        assert!(matches!(
            client.get_access_token(&error_redirect_uri).await.unwrap_err(),
            DigidClientError::RedirectUriError {
                ref error,
                error_description: Some(ref error_description),
            } if error == "error_type" && error_description == "this is the error description"
        ));

        let short_error_redirect_uri = {
            let mut redirect_uri = REDIRECT_URI.clone();

            redirect_uri.query_pairs_mut().append_pair(PARAM_ERROR, "foobar");

            redirect_uri
        };

        assert!(matches!(
            client.get_access_token(&short_error_redirect_uri).await.unwrap_err(),
            DigidClientError::RedirectUriError {
                ref error,
                error_description: None,
            } if error == "foobar"
        ));

        // Test for the error that is returned if the redirect URI does not contain
        // the CSRF token in the `state` query parameter.

        let wrong_csrf_redirect_uri = {
            let mut redirect_uri = REDIRECT_URI.clone();

            redirect_uri.query_pairs_mut().append_pair(PARAM_STATE, "foobar");

            redirect_uri
        };

        assert!(matches!(
            client.get_access_token(&wrong_csrf_redirect_uri).await.unwrap_err(),
            DigidClientError::StateTokenMismatch
        ));

        // Test for the error that is returned if the redirect URI does not have
        // a `code` query parameter.

        let no_auth_code_redirect_uri = {
            let mut redirect_uri = REDIRECT_URI.clone();

            redirect_uri
                .query_pairs_mut()
                .append_pair(PARAM_STATE, &generated_csrf_token);
            redirect_uri
        };

        assert!(matches!(
            client.get_access_token(&no_auth_code_redirect_uri).await.unwrap_err(),
            DigidClientError::NoAuthCode
        ));

        // Finally we can test the successful call to `DigidClient.get_access_token()`.
        // First, generate the correct redirect URI.

        let redirect_uri = {
            let mut redirect_uri = REDIRECT_URI.clone();

            redirect_uri
                .query_pairs_mut()
                .append_pair(PARAM_STATE, &generated_csrf_token)
                .append_pair(PARAM_CODE, AUTH_CODE);

            redirect_uri
        };

        // Then, set up the mock to respond when called with the correct parameters.

        client
            .session_state
            .as_mut()
            .unwrap()
            .openid_client
            .expect_authenticate()
            .with(eq(AUTH_CODE), eq(generated_nonce), eq(PKCE_VERIFIER))
            .once()
            .returning(|_, _, _| Ok(ACCESS_CODE.to_string()));

        // Call `DigidClient.get_access_token()` ...

        let access_code = client
            .get_access_token(&redirect_uri)
            .await
            .expect("Could not get DigiD access token");

        // ... and check the result and internal state.
        assert_eq!(access_code, ACCESS_CODE);
        assert!(client.session_state.is_none());

        // Now that the session is cleared interally, calling `DigidClient.get_access_token()`
        // again should result in an error.
        assert!(matches!(
            client.get_access_token(&redirect_uri).await.unwrap_err(),
            DigidClientError::NoSession
        ));

        // Also, a valid redirect URI should not longer be accepted.
        assert!(!client.accepts_redirect_uri(Lazy::force(&REDIRECT_URI)));
    }
}
