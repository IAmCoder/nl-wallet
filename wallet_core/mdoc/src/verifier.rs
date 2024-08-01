//! RP software, for verifying mdoc disclosures, see [`DeviceResponse::verify()`].

use std::{collections::HashMap, time::Duration};

use chrono::{DateTime, Utc};
use derive_more::AsRef;
use indexmap::IndexMap;
use nutype::nutype;
use p256::SecretKey;
use serde::{Deserialize, Serialize};
use strfmt::strfmt;
use tracing::{debug, warn};
use url::Url;
use webpki::TrustAnchor;

use wallet_common::generator::Generator;

use crate::{
    identifiers::{AttributeIdentifier, AttributeIdentifierHolder},
    iso::*,
    server_keys::KeyPair,
    server_state::{SessionStoreError, SessionToken},
    unsigned::Entry,
    utils::{
        cose::ClonePayload,
        crypto::{cbor_digest, dh_hmac_key},
        serialization::{cbor_serialize, CborSeq, TaggedBytes},
        x509::CertificateUsage,
    },
    Result,
};

/// Attributes of an mdoc that was disclosed in a [`DeviceResponse`], as computed by [`DeviceResponse::verify()`].
/// Grouped per namespace. Validity information and the attributes issuer's common_name is also included.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DocumentDisclosedAttributes {
    pub attributes: IndexMap<NameSpace, Vec<Entry>>,
    pub issuer: Vec<String>,
    pub validity_info: ValidityInfo,
}
/// All attributes that were disclosed in a [`DeviceResponse`], as computed by [`DeviceResponse::verify()`].
pub type DisclosedAttributes = IndexMap<DocType, DocumentDisclosedAttributes>;

#[derive(thiserror::Error, Debug)]
pub enum VerificationError {
    #[error("errors in device response: {0:#?}")]
    DeviceResponseErrors(Vec<DocumentError>),
    #[error("unexpected status: {0}")]
    UnexpectedStatus(u64),
    #[error("no documents found in device response")]
    NoDocuments,
    #[error("inconsistent doctypes: document contained {document}, mso contained {mso}")]
    WrongDocType { document: DocType, mso: DocType },
    #[error("namespace {0} not found in mso")]
    MissingNamespace(NameSpace),
    #[error("digest ID {0} not found in mso")]
    MissingDigestID(DigestID),
    #[error("attribute verification failed: did not hash to the value in the MSO")]
    AttributeVerificationFailed,
    #[error("missing ephemeral key")]
    EphemeralKeyMissing,
    #[error("validity error: {0}")]
    Validity(#[from] ValidityError),
    #[error("missing OriginInfo in engagement: {0}")]
    MissingOriginInfo(usize),
    #[error("incorrect OriginInfo in engagement")]
    IncorrectOriginInfo,
    #[error("missing verifier URL params")]
    MissingVerifierUrlParameters,
    #[error("unknown use case: {0}")]
    UnknownUseCase(String),
    #[error("presence or absence of return url template does not match configuration for the required use case")]
    ReturnUrlConfigurationMismatch,
    #[error("unknown session ID: {0}")]
    UnknownSessionId(SessionToken),
    #[error("no ItemsRequest: can't request a disclosure of 0 attributes")]
    NoItemsRequests,
    #[error("attributes mismatch: {0:?}")]
    MissingAttributes(Vec<AttributeIdentifier>),
    #[error("error with sessionstore: {0}")]
    SessionStore(#[source] SessionStoreError),
    #[error("disclosed attributes requested for disclosure session with status other than 'Done'")]
    SessionNotDone,
    #[error("return URL nonce not provided")]
    ReturnUrlNonceMissing,
    #[error("return URL nonce '{0}' does not match expected")]
    ReturnUrlNonceMismatch(String),
    #[error("the ephemeral ID {} is invalid", hex::encode(.0))]
    InvalidEphemeralId(Vec<u8>),
    #[error("the ephemeral ID {} has expired", hex::encode(.0))]
    ExpiredEphemeralId(Vec<u8>),
    #[error("URL encoding error: {0}")]
    UrlEncoding(#[from] serde_urlencoded::ser::Error),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, AsRef)]
pub struct ItemsRequests(pub Vec<ItemsRequest>);
impl From<Vec<ItemsRequest>> for ItemsRequests {
    fn from(value: Vec<ItemsRequest>) -> Self {
        Self(value)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SessionType {
    // Using Universal Link
    SameDevice,
    /// Using QR code
    CrossDevice,
}

pub const EPHEMERAL_ID_VALIDITY_SECONDS: Duration = Duration::from_secs(10);

#[nutype(
    derive(Debug, Clone, FromStr, Serialize, Deserialize),
    validate(predicate = ReturnUrlTemplate::is_valid_return_url_template),
)]
pub struct ReturnUrlTemplate(String);

impl ReturnUrlTemplate {
    pub fn into_url(self, session_token: &SessionToken) -> Url {
        strfmt!(&self.into_inner(), session_token => session_token.to_string())
            .expect("valid ReturnUrlTemplate should always format")
            .parse()
            .expect("formatted ReturnUrlTemplate should always be a valid URL")
    }

    fn is_valid_return_url_template(s: &str) -> bool {
        #[cfg(feature = "allow_http_return_url")]
        const ALLOWED_SCHEMES: [&str; 2] = ["https", "http"];
        #[cfg(not(feature = "allow_http_return_url"))]
        const ALLOWED_SCHEMES: [&str; 1] = ["https"];

        // It should be a valid URL when removing the template parameter.
        let s = s.replace("{session_token}", "");
        let url = s.parse::<Url>();

        url.is_ok_and(|url| ALLOWED_SCHEMES.contains(&url.scheme()))
    }
}

#[nutype(derive(Debug, From, AsRef))]
pub struct UseCases(HashMap<String, UseCase>);

#[derive(Debug)]
pub struct UseCase {
    pub key_pair: KeyPair,
    pub session_type_return_url: SessionTypeReturnUrl,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionTypeReturnUrl {
    Neither,
    #[default]
    SameDevice,
    Both,
}

impl ReaderEngagement {
    pub fn try_new(privkey: &SecretKey, verifier_url: Url) -> Result<Self> {
        let engagement = Engagement {
            version: EngagementVersion::V1_0,
            security: Some((&privkey.public_key()).try_into()?),
            connection_methods: Some(vec![ConnectionMethodKeyed {
                typ: ConnectionMethodType::RestApi,
                version: ConnectionMethodVersion::RestApi,
                connection_options: RestApiOptionsKeyed { uri: verifier_url }.into(),
            }
            .into()]),
            origin_infos: vec![],
        };

        Ok(engagement.into())
    }
}

impl ItemsRequests {
    /// Checks that all `requested` attributes are disclosed in this [`DeviceResponse`].
    pub fn match_against_response(&self, device_response: &DeviceResponse) -> Result<()> {
        let not_found: Vec<_> = self
            .0
            .iter()
            .flat_map(|items_request| {
                device_response
                    .documents
                    .as_ref()
                    .and_then(|docs| docs.iter().find(|doc| doc.doc_type == items_request.doc_type))
                    .map_or_else(
                        // If the entire document is missing then all requested attributes are missing
                        || items_request.attribute_identifiers().into_iter().collect(),
                        |doc| items_request.match_against_issuer_signed(doc),
                    )
            })
            .collect();

        if not_found.is_empty() {
            Ok(())
        } else {
            Err(VerificationError::MissingAttributes(not_found).into())
        }
    }
}

impl DeviceResponse {
    /// Verify a [`DeviceResponse`], returning the verified attributes, grouped per doctype and namespace.
    ///
    /// # Arguments
    /// - `eph_reader_key` - the ephemeral reader public key in case the mdoc is authentication with a MAC.
    /// - `device_authentication_bts` - the [`DeviceAuthenticationBytes`] acting as the challenge, i.e., that have
    ///   to be signed by the holder.
    /// - `time` - a generator of the current time.
    /// - `trust_anchors` - trust anchors against which verification is done.
    pub fn verify(
        &self,
        eph_reader_key: Option<&SecretKey>,
        session_transcript: &SessionTranscript,
        time: &impl Generator<DateTime<Utc>>,
        trust_anchors: &[TrustAnchor],
    ) -> Result<DisclosedAttributes> {
        if let Some(errors) = &self.document_errors {
            return Err(VerificationError::DeviceResponseErrors(errors.clone()).into());
        }
        if self.status != 0 {
            return Err(VerificationError::UnexpectedStatus(self.status).into());
        }

        if self.documents.is_none() {
            return Err(VerificationError::NoDocuments.into());
        }

        let mut attrs = IndexMap::new();
        for doc in self.documents.as_ref().unwrap() {
            debug!("verifying document with doc_type: {}", doc.doc_type);
            let (doc_type, doc_attrs) = doc
                .verify(eph_reader_key, session_transcript, time, trust_anchors)
                .map_err(|e| {
                    warn!("document verification failed: {e}");
                    e
                })?;
            attrs.insert(doc_type, doc_attrs);
            debug!("document OK");
        }

        Ok(attrs)
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidityError {
    #[error("validity parsing failed: {0}")]
    ParsingFailed(#[from] chrono::ParseError),
    #[error("not yet valid: valid from {0}")]
    NotYetValid(String),
    #[error("expired at {0}")]
    Expired(String),
}

/// Indicate how a [`ValidityInfo`] should be verified against the current date.
pub enum ValidityRequirement {
    /// The [`ValidityInfo`] must not be expired, but it is allowed to be not yet valid.
    AllowNotYetValid,
    /// The [`ValidityInfo`] must be valid now and not be expired.
    Valid,
}

impl ValidityInfo {
    pub fn verify_is_valid_at(
        &self,
        time: DateTime<Utc>,
        validity: ValidityRequirement,
    ) -> std::result::Result<(), ValidityError> {
        if matches!(validity, ValidityRequirement::Valid) && time < DateTime::<Utc>::try_from(&self.valid_from)? {
            Err(ValidityError::NotYetValid(self.valid_from.0 .0.clone()))
        } else if time > DateTime::<Utc>::try_from(&self.valid_until)? {
            Err(ValidityError::Expired(self.valid_from.0 .0.clone()))
        } else {
            Ok(())
        }
    }
}

impl IssuerSigned {
    pub fn verify(
        &self,
        validity: ValidityRequirement,
        time: &impl Generator<DateTime<Utc>>,
        trust_anchors: &[TrustAnchor],
    ) -> Result<(DocumentDisclosedAttributes, MobileSecurityObject)> {
        let TaggedBytes(mso) =
            self.issuer_auth
                .verify_against_trust_anchors(CertificateUsage::Mdl, time, trust_anchors)?;

        mso.validity_info
            .verify_is_valid_at(time.generate(), validity)
            .map_err(VerificationError::Validity)?;

        let attrs = self
            .name_spaces
            .as_ref()
            .map(|name_spaces| {
                name_spaces
                    .as_ref()
                    .iter()
                    .map(|(namespace, items)| Ok((namespace.clone(), mso.verify_attrs_in_namespace(items, namespace)?)))
                    .collect::<Result<_>>()
            })
            .transpose()?
            .unwrap_or_default();

        Ok((
            DocumentDisclosedAttributes {
                attributes: attrs,
                issuer: self.issuer_auth.signing_cert()?.iter_common_name()?,
                validity_info: mso.validity_info.clone(),
            },
            mso,
        ))
    }
}

impl MobileSecurityObject {
    fn verify_attrs_in_namespace(&self, attrs: &Attributes, namespace: &NameSpace) -> Result<Vec<Entry>> {
        attrs
            .as_ref()
            .iter()
            .map(|item| {
                self.verify_attr_digest(namespace, item)?;
                Ok(Entry {
                    name: item.0.element_identifier.clone(),
                    value: item.0.element_value.clone(),
                })
            })
            .collect::<Result<_>>()
    }

    /// Given an `IssuerSignedItem` i.e. an attribute, verify that its digest is correctly included in the MSO.
    fn verify_attr_digest(&self, namespace: &NameSpace, item: &IssuerSignedItemBytes) -> Result<()> {
        let digest_id = item.0.digest_id;
        let digest = self
            .value_digests
            .0
            .get(namespace)
            .ok_or_else(|| VerificationError::MissingNamespace(namespace.clone()))?
            .0
            .get(&digest_id)
            .ok_or_else(|| VerificationError::MissingDigestID(digest_id))?;
        if *digest != cbor_digest(item)? {
            return Err(VerificationError::AttributeVerificationFailed.into());
        }
        Ok(())
    }
}

impl Document {
    pub fn verify(
        &self,
        eph_reader_key: Option<&SecretKey>,
        session_transcript: &SessionTranscript,
        time: &impl Generator<DateTime<Utc>>,
        trust_anchors: &[TrustAnchor],
    ) -> Result<(DocType, DocumentDisclosedAttributes)> {
        debug!("verifying document with doc_type: {:?}", &self.doc_type);
        debug!("verify issuer_signed");
        let (attrs, mso) = self
            .issuer_signed
            .verify(ValidityRequirement::Valid, time, trust_anchors)?;

        debug!("verifying mso.doc_type matches document doc_type");
        if self.doc_type != mso.doc_type {
            return Err(VerificationError::WrongDocType {
                document: self.doc_type.clone(),
                mso: mso.doc_type,
            }
            .into());
        }

        debug!("serializing session transcript");
        let session_transcript_bts = cbor_serialize(&TaggedBytes(session_transcript))?;
        let device_authentication = DeviceAuthenticationKeyed::new(&self.doc_type, session_transcript);
        debug!("serializing device_authentication");
        let device_authentication_bts = cbor_serialize(&TaggedBytes(CborSeq(device_authentication)))?;

        debug!("extracting device_key");
        let device_key = (&mso.device_key_info.device_key).try_into()?;
        match &self.device_signed.device_auth {
            DeviceAuth::DeviceSignature(sig) => {
                debug!("verifying DeviceSignature");
                sig.clone_with_payload(device_authentication_bts.to_vec())
                    .verify(&device_key)?;
            }
            DeviceAuth::DeviceMac(mac) => {
                debug!("verifying DeviceMac");
                let mac_key = dh_hmac_key(
                    eph_reader_key.ok_or_else(|| VerificationError::EphemeralKeyMissing)?,
                    &device_key.into(),
                    &session_transcript_bts,
                    "EMacKey",
                    32,
                )?;
                mac.clone_with_payload(device_authentication_bts.to_vec())
                    .verify(&mac_key)?;
            }
        }
        debug!("signature valid");

        Ok((mso.doc_type, attrs))
    }
}

impl ItemsRequest {
    /// Returns requested attributes, if any, that are not present in the `issuer_signed`.
    pub fn match_against_issuer_signed(&self, document: &Document) -> Vec<AttributeIdentifier> {
        let document_identifiers = document.issuer_signed_attribute_identifiers();
        self.attribute_identifiers()
            .into_iter()
            .filter(|attribute| !document_identifiers.contains(attribute))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Add;

    use chrono::{Duration, Utc};
    use rstest::rstest;

    use crate::{
        examples::{
            Example, Examples, IsoCertTimeGenerator, EXAMPLE_ATTR_NAME, EXAMPLE_ATTR_VALUE, EXAMPLE_DOC_TYPE,
            EXAMPLE_NAMESPACE,
        },
        identifiers::AttributeIdentifierHolder,
        test::{self, DebugCollapseBts},
        DeviceAuthenticationBytes, DeviceResponse, Document, Error, ValidityInfo,
    };

    use super::*;

    fn new_validity_info(add_from_days: i64, add_until_days: i64) -> ValidityInfo {
        let now = Utc::now();
        ValidityInfo {
            signed: now.into(),
            valid_from: now.add(Duration::days(add_from_days)).into(),
            valid_until: now.add(Duration::days(add_until_days)).into(),
            expected_update: None,
        }
    }

    #[test]
    fn validity_info() {
        let now = Utc::now();

        let validity = new_validity_info(-1, 1);
        validity.verify_is_valid_at(now, ValidityRequirement::Valid).unwrap();
        validity
            .verify_is_valid_at(now, ValidityRequirement::AllowNotYetValid)
            .unwrap();

        let validity = new_validity_info(-2, -1);
        assert!(matches!(
            validity.verify_is_valid_at(now, ValidityRequirement::Valid),
            Err(ValidityError::Expired(_))
        ));
        assert!(matches!(
            validity.verify_is_valid_at(now, ValidityRequirement::AllowNotYetValid),
            Err(ValidityError::Expired(_))
        ));

        let validity = new_validity_info(1, 2);
        assert!(matches!(
            validity.verify_is_valid_at(now, ValidityRequirement::Valid),
            Err(ValidityError::NotYetValid(_))
        ));
        validity
            .verify_is_valid_at(now, ValidityRequirement::AllowNotYetValid)
            .unwrap();
    }

    /// Verify the example disclosure from the standard.
    #[test]
    fn verify_iso_example_disclosure() {
        let device_response = DeviceResponse::example();
        println!("DeviceResponse: {:#?} ", DebugCollapseBts::from(&device_response));

        // Examine the first attribute in the device response
        let document = device_response.documents.as_ref().unwrap()[0].clone();
        assert_eq!(document.doc_type, EXAMPLE_DOC_TYPE);
        let namespaces = document.issuer_signed.name_spaces.as_ref().unwrap();
        let attrs = namespaces.as_ref().get(EXAMPLE_NAMESPACE).unwrap();
        let issuer_signed_attr = attrs.as_ref().first().unwrap().0.clone();
        assert_eq!(issuer_signed_attr.element_identifier, EXAMPLE_ATTR_NAME);
        assert_eq!(issuer_signed_attr.element_value, *EXAMPLE_ATTR_VALUE);
        println!("issuer_signed_attr: {:#?}", DebugCollapseBts::from(&issuer_signed_attr));

        // Do the verification
        let eph_reader_key = Examples::ephemeral_reader_key();
        let trust_anchors = Examples::iaca_trust_anchors();
        let disclosed_attrs = device_response
            .verify(
                Some(&eph_reader_key),
                &DeviceAuthenticationBytes::example().0 .0.session_transcript, // To be signed by device key found in MSO
                &IsoCertTimeGenerator,
                trust_anchors,
            )
            .unwrap();
        println!("DisclosedAttributes: {:#?}", DebugCollapseBts::from(&disclosed_attrs));

        // The first disclosed attribute is the same as we saw earlier in the DeviceResponse
        test::assert_disclosure_contains(
            &disclosed_attrs,
            EXAMPLE_DOC_TYPE,
            EXAMPLE_NAMESPACE,
            EXAMPLE_ATTR_NAME,
            &EXAMPLE_ATTR_VALUE,
        );
    }

    #[rstest]
    #[case(do_nothing())]
    #[case(swap_attributes())]
    #[case(remove_documents())]
    #[case(remove_document())]
    #[case(change_doctype())]
    #[case(change_namespace())]
    #[case(remove_attribute())]
    #[case(multiple_doc_types_swapped())]
    fn match_disclosed_attributes(
        #[case] testcase: (DeviceResponse, ItemsRequests, Result<(), Vec<AttributeIdentifier>>),
    ) {
        // Construct an items request that matches the example device response
        let (device_response, items_requests, expected_result) = testcase;
        assert_eq!(
            items_requests
                .match_against_response(&device_response)
                .map_err(|e| match e {
                    Error::Verification(VerificationError::MissingAttributes(e)) => e,
                    _ => panic!(),
                }),
            expected_result,
        );
    }

    /// Helper to compute all attribute identifiers contained in a bunch of [`ItemsRequest`]s.
    fn attribute_identifiers(items_requests: &ItemsRequests) -> Vec<AttributeIdentifier> {
        items_requests
            .0
            .iter()
            .flat_map(AttributeIdentifierHolder::attribute_identifiers)
            .collect()
    }

    // return an unmodified device response, which should verify
    fn do_nothing() -> (DeviceResponse, ItemsRequests, Result<(), Vec<AttributeIdentifier>>) {
        (DeviceResponse::example(), Examples::items_requests(), Ok(()))
    }

    // Matching attributes is insensitive to swapped attributes, so verification succeeds
    fn swap_attributes() -> (DeviceResponse, ItemsRequests, Result<(), Vec<AttributeIdentifier>>) {
        let mut device_response = DeviceResponse::example();
        let first_document = device_response.documents.as_mut().unwrap().first_mut().unwrap();
        let name_spaces = first_document.issuer_signed.name_spaces.as_mut().unwrap();

        name_spaces.modify_first_attributes(|attributes| {
            attributes.swap(0, 1);
        });

        (device_response, Examples::items_requests(), Ok(()))
    }

    // remove all disclosed documents
    fn remove_documents() -> (DeviceResponse, ItemsRequests, Result<(), Vec<AttributeIdentifier>>) {
        let mut device_response = DeviceResponse::example();
        device_response.documents = None;

        let items_requests = Examples::items_requests();
        let missing = attribute_identifiers(&items_requests);
        (device_response, items_requests, Err(missing))
    }

    // remove a single disclosed document
    fn remove_document() -> (DeviceResponse, ItemsRequests, Result<(), Vec<AttributeIdentifier>>) {
        let mut device_response = DeviceResponse::example();
        device_response.documents.as_mut().unwrap().pop();

        let items_requests = Examples::items_requests();
        let missing = attribute_identifiers(&items_requests);
        (device_response, items_requests, Err(missing))
    }

    // Change the first doctype so it is not the requested one
    fn change_doctype() -> (DeviceResponse, ItemsRequests, Result<(), Vec<AttributeIdentifier>>) {
        let mut device_response = DeviceResponse::example();
        device_response
            .documents
            .as_mut()
            .unwrap()
            .first_mut()
            .unwrap()
            .doc_type = "some_not_requested_doc_type".to_string();

        let items_requests = Examples::items_requests();
        let missing = attribute_identifiers(&items_requests);
        (device_response, items_requests, Err(missing))
    }

    // Change a namespace so it is not the requested one
    fn change_namespace() -> (DeviceResponse, ItemsRequests, Result<(), Vec<AttributeIdentifier>>) {
        let mut device_response = DeviceResponse::example();
        let first_document = device_response.documents.as_mut().unwrap().first_mut().unwrap();
        let name_spaces = first_document.issuer_signed.name_spaces.as_mut().unwrap();

        name_spaces.modify_namespaces(|name_spaces| {
            let (_, attributes) = name_spaces.pop().unwrap();
            name_spaces.insert("some_not_requested_name_space".to_string(), attributes);
        });

        let items_requests = Examples::items_requests();
        let missing = attribute_identifiers(&items_requests);
        (device_response, items_requests, Err(missing))
    }

    // Remove one of the disclosed attributes
    fn remove_attribute() -> (DeviceResponse, ItemsRequests, Result<(), Vec<AttributeIdentifier>>) {
        let mut device_response = DeviceResponse::example();
        let first_document = device_response.documents.as_mut().unwrap().first_mut().unwrap();
        let name_spaces = first_document.issuer_signed.name_spaces.as_mut().unwrap();

        name_spaces.modify_first_attributes(|attributes| {
            attributes.pop();
        });

        let items_requests = Examples::items_requests();
        let missing = vec![attribute_identifiers(&items_requests).last().unwrap().clone()];
        (device_response, items_requests, Err(missing))
    }

    // Add one extra document with doc_type "a", and swap the order in the items_requests
    fn multiple_doc_types_swapped() -> (DeviceResponse, ItemsRequests, Result<(), Vec<AttributeIdentifier>>) {
        let mut device_response = DeviceResponse::example();
        let mut cloned_doc: Document = device_response.documents.as_ref().unwrap()[0].clone();
        cloned_doc.doc_type = "a".to_string();
        device_response.documents.as_mut().unwrap().push(cloned_doc);

        let mut items_requests = Examples::items_requests();
        let mut cloned_items_request = items_requests.0[0].clone();
        cloned_items_request.doc_type = "a".to_string();
        items_requests.0.push(cloned_items_request);

        // swap the document order in items_requests
        items_requests.0.reverse();

        (device_response, items_requests, Ok(()))
    }

    #[rstest]
    #[case("https://example.com/{session_token}", true)]
    #[case("https://example.com/return/{session_token}", true)]
    #[case("https://example.com/return/{session_token}/url", true)]
    #[case("https://example.com/{session_token}/", true)]
    #[case("https://example.com/return/{session_token}/", true)]
    #[case("https://example.com/return/{session_token}/url/", true)]
    #[case("https://example.com/return/{session_token}?hello=world&bye=mars#hashtag", true)]
    #[case("https://example.com/{session_token}/{session_token}", true)]
    #[case("https://example.com/", true)]
    #[case("https://example.com/return", true)]
    #[case("https://example.com/return/url", true)]
    #[case("https://example.com/return/", true)]
    #[case("https://example.com/return/url/", true)]
    #[case("https://example.com/return/?hello=world&bye=mars#hashtag", true)]
    #[case("https://example.com/{session_token}/{not_session_token}", true)]
    #[case("file://etc/passwd", false)]
    #[case("file://etc/{session_token}", false)]
    #[case("https://{session_token}", false)]
    #[cfg_attr(feature = "allow_http_return_url", case("http://example.com/{session_token}", true))]
    #[cfg_attr(
        not(feature = "allow_http_return_url"),
        case("http://example.com/{session_token}", false)
    )]
    fn test_return_url_template(#[case] return_url_string: String, #[case] should_parse: bool) {
        assert_eq!(return_url_string.parse::<ReturnUrlTemplate>().is_ok(), should_parse);
        assert_eq!(
            ReturnUrlTemplate::is_valid_return_url_template(&return_url_string),
            should_parse
        )
    }
}
