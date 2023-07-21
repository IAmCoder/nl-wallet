use core::fmt::Debug;
use std::{ops::Add, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use ciborium::value::Value;
use indexmap::IndexMap;
use once_cell::sync::OnceCell;
use serde::{de::DeserializeOwned, Serialize};

use nl_wallet_mdoc::{
    basic_sa_ext::{Entry, RequestKeyGenerationMessage, UnsignedMdoc},
    holder::*,
    iso::*,
    issuer::*,
    issuer_shared::SessionToken,
    utils::{
        serialization::{cbor_deserialize, cbor_serialize},
        signer::SoftwareEcdsaKey,
        time::mock_time::{clear_mock_time, set_mock_time},
        x509::{Certificate, CertificateUsage},
    },
    Error,
};

mod examples;
use examples::*;

mod mdocs_map;
use mdocs_map::MdocsMap;

/// Verify that the static device key example from the spec is the public key in the MSO.
#[test]
fn iso_examples_consistency() {
    // Some of the certificates in the ISO examples are valid from Oct 1, 2020 to Oct 1, 2021.
    set_mock_time(Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap());

    let static_device_key = Examples::static_device_key();

    let device_key = &DeviceResponse::example().documents.unwrap()[0]
        .issuer_signed
        .issuer_auth
        .verify_against_trust_anchors(CertificateUsage::Mdl, &Examples::issuer_ca_cert())
        .unwrap()
        .0
         .0
        .device_key_info
        .device_key;

    assert_eq!(
        static_device_key.verifying_key(),
        ecdsa::VerifyingKey::<p256::NistP256>::try_from(device_key).unwrap(),
    );

    clear_mock_time();
}

#[test]
fn iso_examples_disclosure() {
    // Some of the certificates in the ISO examples are valid from Oct 1, 2020 to Oct 1, 2021.
    set_mock_time(Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap());

    let ca_cert = Examples::issuer_ca_cert();
    let eph_reader_key = Examples::ephemeral_reader_key();
    let device_response = DeviceResponse::example();
    println!("DeviceResponse: {:#?} ", DebugCollapseBts(&device_response));

    // Do the verification
    let disclosed_attributes = device_response
        .verify(
            Some(&eph_reader_key),
            &DeviceAuthenticationBytes::example_bts(), // To be signed by device key found in MSO
            &ca_cert,
        )
        .unwrap();
    println!("DisclosedAttributes: {:#?}", DebugCollapseBts(disclosed_attributes));

    let device_request = DeviceRequest::example();
    println!("DeviceRequest: {:#?}", DebugCollapseBts(&device_request));

    let reader_ca_cert = Examples::reader_ca_cert();
    println!(
        "Reader: {:#?}",
        device_request
            .verify(&reader_ca_cert, &ReaderAuthenticationBytes::example_bts())
            .unwrap(),
    );

    let static_device_key = Examples::static_device_key();
    SoftwareEcdsaKey::insert("example_static_device_key", static_device_key);
    let cred = Mdoc::<SoftwareEcdsaKey>::new(
        "example_static_device_key".to_string(),
        device_response.documents.as_ref().unwrap()[0].issuer_signed.clone(),
        &ca_cert,
    )
    .unwrap();

    let wallet = Wallet::new(MdocsMap::try_from([cred]).unwrap());
    let resp = wallet
        .disclose::<SoftwareEcdsaKey>(&device_request, &DeviceAuthenticationBytes::example_bts())
        .unwrap();

    println!("DeviceResponse: {:#?}", DebugCollapseBts(&resp));
    println!(
        "Disclosure: {:#?}",
        DebugCollapseBts(resp.verify(None, &DeviceAuthenticationBytes::example_bts(), &ca_cert)),
    );

    clear_mock_time();
}

#[test]
fn iso_examples_custom_disclosure() {
    // Some of the certificates in the ISO examples are valid from Oct 1, 2020 to Oct 1, 2021.
    set_mock_time(Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap());

    let ca_cert = Examples::issuer_ca_cert();
    let device_response = DeviceResponse::example();

    let request = DeviceRequest::new(vec![ItemsRequest {
        doc_type: "org.iso.18013.5.1.mDL".to_string(),
        name_spaces: IndexMap::from([(
            "org.iso.18013.5.1".to_string(),
            IndexMap::from([("family_name".to_string(), false)]),
        )]),
        request_info: None,
    }]);
    println!("My Request: {:#?}", DebugCollapseBts(&request));

    let static_device_key = Examples::static_device_key();
    SoftwareEcdsaKey::insert("example_static_device_key", static_device_key);
    let cred = Mdoc::<SoftwareEcdsaKey>::new(
        "example_static_device_key".to_string(),
        device_response.documents.as_ref().unwrap()[0].issuer_signed.clone(),
        &ca_cert,
    )
    .unwrap();

    let wallet = Wallet::new(MdocsMap::try_from([cred]).unwrap());
    let resp = wallet
        .disclose::<SoftwareEcdsaKey>(&request, &DeviceAuthenticationBytes::example_bts())
        .unwrap();

    println!("My DeviceResponse: {:#?}", DebugCollapseBts(&resp));
    println!(
        "My Disclosure: {:#?}",
        DebugCollapseBts(resp.verify(None, &DeviceAuthenticationBytes::example_bts(), &ca_cert)),
    );

    clear_mock_time();
}

const ISSUANCE_CA_CN: &str = "ca.issuer.example.com";
const ISSUANCE_CERT_CN: &str = "cert.issuer.example.com";
const ISSUANCE_DOC_TYPE: &str = "example_doctype";
const ISSUANCE_NAME_SPACE: &str = "example_namespace";
const ISSUANCE_ATTRS: [(&str, &str); 2] = [("first_name", "John"), ("family_name", "Doe")];

fn new_issuance_request() -> Vec<UnsignedMdoc> {
    vec![UnsignedMdoc {
        doc_type: ISSUANCE_DOC_TYPE.to_string(),
        count: 2,
        valid_until: chrono::Utc::now().add(chrono::Duration::days(365)).into(),
        attributes: IndexMap::from([(
            ISSUANCE_NAME_SPACE.to_string(),
            ISSUANCE_ATTRS
                .iter()
                .map(|(key, val)| Entry {
                    name: key.to_string(),
                    value: Value::Text(val.to_string()),
                })
                .collect(),
        )]),
    }]
}

struct MockHttpClient<'a, K, S> {
    issuance_server: &'a Server<K, S>,
    session_token: SessionToken,
}

#[async_trait]
impl HttpClient for MockHttpClient<'_, MockIssuanceKeyring, MemorySessionStore> {
    async fn post<R, V>(&self, val: &V) -> Result<R, Error>
    where
        V: Serialize + Sync,
        R: DeserializeOwned,
    {
        let response = self
            .issuance_server
            .process_message(self.session_token.clone(), cbor_serialize(val).unwrap())
            .unwrap();

        // Hacky way to cast `response`, which is a `Box<dyn IssuerResponse>`, to the requested type:
        // serialize to CBOR and back
        let response = cbor_deserialize(cbor_serialize(&response).unwrap().as_slice()).unwrap();
        Ok(response)
    }
}

struct MockHttpClientBuilder<'a, K, S> {
    issuance_server: &'a Server<K, S>,
}

impl<'a> HttpClientBuilder for MockHttpClientBuilder<'a, MockIssuanceKeyring, MemorySessionStore> {
    type Client = MockHttpClient<'a, MockIssuanceKeyring, MemorySessionStore>;
    fn build(&self, engagement: ServiceEngagement) -> Self::Client {
        // Strip off leading /
        let url = engagement.url.unwrap()[1..].to_string();

        MockHttpClient {
            issuance_server: self.issuance_server,
            session_token: url.into(),
        }
    }
}

struct MockIssuanceKeyring {
    issuance_key: PrivateKey,
}
impl KeyRing for MockIssuanceKeyring {
    fn private_key(&self, _: &DocType) -> Option<&PrivateKey> {
        Some(&self.issuance_key)
    }
}

fn user_consent_async<const CONSENT: bool>() -> impl IssuanceUserConsent {
    struct MockUserConsent<const CONSENT: bool>;
    #[async_trait]
    impl<const CONSENT: bool> IssuanceUserConsent for MockUserConsent<CONSENT> {
        async fn ask(&self, _: &RequestKeyGenerationMessage) -> bool {
            CONSENT
        }
    }
    MockUserConsent::<CONSENT>
}

fn user_consent_without_async() -> impl IssuanceUserConsent {
    #[derive(Default, Clone)]
    struct MockIssuanceSessionReceiver {
        /// Keep track of the `IssuanceSessions` as we need to invoke `provide_consent()` on it.
        /// This is a `OnceCell` because we have to instantiate this struct before the `IssuanceSessions`,
        /// while in `receive()` below we have to refer that same `IssuanceSessions`.
        ///
        /// In real-world implementations of `IssuanceSessionReceiver`, receiving session requests and
        /// providing consent for a session is expected to be be much less tightly coupled, since the latter happens by
        /// user initiative.
        sessions: Arc<OnceCell<IssuanceSessions<MockIssuanceSessionReceiver>>>,
    }
    impl IssuanceSessionReceiver for MockIssuanceSessionReceiver {
        fn receive(&self, msg: &RequestKeyGenerationMessage) {
            let sessions = self.sessions.get().unwrap();
            sessions.provide_consent(&msg.e_session_id, true)
        }
    }

    #[async_trait]
    impl IssuanceUserConsent for MockIssuanceSessionReceiver {
        async fn ask(&self, msg: &RequestKeyGenerationMessage) -> bool {
            self.sessions.get().unwrap().ask(msg).await
        }
    }

    let user_consent = MockIssuanceSessionReceiver::default();
    let sessions = IssuanceSessions::new(user_consent.clone());
    assert!(user_consent.sessions.set(sessions).is_ok());

    user_consent
}

#[test]
fn issuance_and_disclosure() {
    let (wallet, ca) = issuance_and_disclosure_using_consent(user_consent_without_async());
    custom_disclosure(wallet, ca);

    let (wallet, ca) = issuance_and_disclosure_using_consent(user_consent_async::<true>());
    custom_disclosure(wallet, ca);

    let (wallet, _) = issuance_and_disclosure_using_consent(user_consent_async::<false>());
    assert!(wallet.list_mdocs::<SoftwareEcdsaKey>().is_empty())
}

fn issuance_and_disclosure_using_consent<T: IssuanceUserConsent>(user_consent: T) -> (Wallet<MdocsMap>, Certificate) {
    // Issuer CA certificate and normal certificate
    let (ca, ca_privkey) = Certificate::new_ca(ISSUANCE_CA_CN).unwrap();
    let (issuer_cert, issuer_privkey) =
        Certificate::new(&ca, &ca_privkey, ISSUANCE_CERT_CN, CertificateUsage::Mdl).unwrap();
    let issuance_key = PrivateKey::new(issuer_privkey, issuer_cert.as_bytes().into());

    // Setup session and issuer
    let request = new_issuance_request();
    let issuance_server = Server::new(
        "".to_string(),
        MockIssuanceKeyring { issuance_key },
        MemorySessionStore::new(),
    );
    let service_engagement = issuance_server.new_session(request).unwrap();

    // Setup holder
    let wallet = Wallet::new(MdocsMap::new());
    assert!(wallet.list_mdocs::<SoftwareEcdsaKey>().is_empty());

    // Do issuance
    let client_builder = MockHttpClientBuilder {
        issuance_server: &issuance_server,
    };
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(async {
            wallet
                .do_issuance::<SoftwareEcdsaKey>(
                    service_engagement,
                    &user_consent,
                    &client_builder,
                    &[(&ca).try_into().unwrap()].as_slice().into(),
                )
                .await
                .unwrap();
        });

    (wallet, ca)
}

fn custom_disclosure(wallet: Wallet<MdocsMap>, ca: Certificate) {
    assert!(!wallet.list_mdocs::<SoftwareEcdsaKey>().is_empty());

    // Disclose some attributes from our cred
    let request = DeviceRequest::new(vec![ItemsRequest {
        doc_type: ISSUANCE_DOC_TYPE.to_string(),
        name_spaces: IndexMap::from([(
            ISSUANCE_NAME_SPACE.to_string(),
            ISSUANCE_ATTRS.iter().map(|(key, _)| (key.to_string(), false)).collect(),
        )]),
        request_info: None,
    }]);

    let challenge = DeviceAuthenticationBytes::example_bts();
    let disclosed = wallet
        .disclose::<SoftwareEcdsaKey>(&request, challenge.as_ref())
        .unwrap();

    println!(
        "Disclosure: {:#?}",
        DebugCollapseBts(
            disclosed
                .verify(None, &challenge, &[(&ca).try_into().unwrap()].as_slice().into())
                .unwrap()
        )
    );
}

/// Wrapper around `T` that implements `Debug` by using `T`'s implementation,
/// but with byte sequences (which can take a lot of vertical space) replaced with
/// a CBOR diagnostic-like notation.
///
/// Example output:
///
/// ```text
/// Test {
///     a_string: "Hello, World",
///     an_int: 42,
///     a_byte_sequence: h'00012AFF',
/// }
/// ```
///
/// Example code:
/// ```rust
/// #[derive(Debug)]
/// struct Test {
///     a_string: String,
///     an_int: u64,
///     a_byte_sequence: Vec<u8>,
/// }
///
/// let test = Test {
///     a_string: "Hello, World".to_string(),
///     an_int: 42,
///     a_byte_sequence: vec![0, 1, 42, 255],
/// };
///
/// println!("{:#?}", DebugCollapseBts(test));
/// ```
struct DebugCollapseBts<T>(T);

impl<T> Debug for DebugCollapseBts<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Match numbers within square brackets, e.g.: [1, 2, 3]
        let debugstr = format!("{:#?}", self.0);
        let debugstr_collapsed = regex::Regex::new(r"\[\s*(\d,?\s*)+\]").unwrap().replace_all(
            debugstr.as_str(),
            |caps: &regex::Captures| {
                let no_whitespace = remove_whitespace(&caps[0]);
                let trimmed = no_whitespace[1..no_whitespace.len() - 2].to_string(); // Remove square brackets
                if trimmed.split(',').any(|r| r.parse::<u8>().is_err()) {
                    // If any of the numbers don't fit in a u8, just return the numbers without whitespace
                    no_whitespace
                } else {
                    format!(
                        "h'{}'", // CBOR diagnostic-like notation
                        hex::encode(
                            trimmed
                                .split(',')
                                .map(|i| i.parse::<u8>().unwrap())
                                .collect::<Vec<u8>>(),
                        )
                        .to_uppercase()
                    )
                }
            },
        );

        write!(f, "{}", debugstr_collapsed)
    }
}

fn remove_whitespace(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}
