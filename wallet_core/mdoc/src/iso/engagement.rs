//! Data structures used in disclosure for everything that has to be signed with the mdoc's private key.
//! Mainly [`DeviceAuthentication`] and all data structures inside it, which includes a transcript
//! of the session so far.
//!
//! NB. "Device authentication" is not to be confused with the [`DeviceAuth`] data structure in the
//! [`disclosure`](super::disclosure) module (which contains the holder's signature over [`DeviceAuthentication`]
//! defined here).
use std::{borrow::Cow, fmt::Debug};

use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use serde_repr::{Deserialize_repr, Serialize_repr};
use serde_with::skip_serializing_none;

use url::Url;
use wallet_common::{config::wallet_config::BaseUrl, utils::sha256};

use crate::{
    iso::disclosure::*,
    utils::{
        cose::CoseKey,
        serialization::{cbor_serialize, CborIntMap, CborSeq, DeviceAuthenticationString, RequiredValue, TaggedBytes},
    },
    verifier::SessionType,
};

/// The data structure that the holder signs with the mdoc private key when disclosing attributes out of that mdoc.
/// Contains a.o. transcript of the session so far, acting as the challenge in a challenge-response mechanism,
/// and the "device-signed items" ([`DeviceNameSpaces`]): attributes that are signed only by the device, since they
/// are part of this data structure, but not by the issuer (i.e., self asserted attributes).
///
/// This data structure is computed by the holder and the RP during a session, and then signed and verified
/// respectively. It is not otherwise included in other data structures.
pub type DeviceAuthentication<'a> = CborSeq<DeviceAuthenticationKeyed<'a>>;

/// See [`DeviceAuthentication`].
pub type DeviceAuthenticationBytes<'a> = TaggedBytes<DeviceAuthentication<'a>>;

/// See [`DeviceAuthentication`].
// In production code, this struct is never deserialized.
#[cfg_attr(any(test, feature = "examples"), derive(Deserialize))]
#[derive(Serialize, Debug, Clone)]
pub struct DeviceAuthenticationKeyed<'a> {
    pub device_authentication: RequiredValue<DeviceAuthenticationString>,
    pub session_transcript: Cow<'a, SessionTranscript>,
    pub doc_type: Cow<'a, str>,
    pub device_name_spaces_bytes: DeviceNameSpacesBytes,
}

impl<'a> DeviceAuthenticationKeyed<'a> {
    pub fn new(doc_type: &'a str, session_transcript: &'a SessionTranscript) -> Self {
        DeviceAuthenticationKeyed {
            device_authentication: Default::default(),
            session_transcript: Cow::Borrowed(session_transcript),
            doc_type: Cow::Borrowed(doc_type),
            device_name_spaces_bytes: Default::default(),
        }
    }
}

#[cfg_attr(any(test, feature = "examples"), derive(Deserialize))]
#[derive(Debug, Clone, Serialize)]
pub struct SessionTranscriptKeyed {
    pub device_engagement_bytes: Option<DeviceEngagementBytes>,
    pub ereader_key_bytes: Option<ESenderKeyBytes>,
    pub handover: Handover,
}

/// Transcript of the session so far. Used in [`DeviceAuthentication`].
pub type SessionTranscript = CborSeq<SessionTranscriptKeyed>;

#[derive(Debug, thiserror::Error)]
pub enum SessionTranscriptError {
    #[error("reader engagement is missing security information")]
    MissingReaderEngagementSecurity,
}

impl SessionTranscript {
    pub fn new(
        session_type: SessionType,
        reader_engagement: &ReaderEngagement,
        device_engagement: &DeviceEngagement,
    ) -> Result<Self, SessionTranscriptError> {
        let reader_security = reader_engagement
            .0
            .security
            .as_ref()
            .ok_or(SessionTranscriptError::MissingReaderEngagementSecurity)?;

        let transcript = SessionTranscriptKeyed {
            device_engagement_bytes: Some(device_engagement.clone().into()),
            handover: match session_type {
                SessionType::SameDevice => Handover::SchemeHandoverBytes(TaggedBytes(reader_engagement.clone())),
                SessionType::CrossDevice => Handover::QRHandover,
            },
            ereader_key_bytes: Some(reader_security.0.e_sender_key_bytes.clone()),
        }
        .into();

        Ok(transcript)
    }

    pub fn new_oid4vp(response_uri: &BaseUrl, client_id: String, nonce: String, mdoc_nonce: String) -> Self {
        let handover = OID4VPHandover {
            client_id_hash: ByteBuf::from(sha256(&cbor_serialize(&[&client_id, &mdoc_nonce]).unwrap())),
            response_uri_hash: ByteBuf::from(sha256(
                &cbor_serialize(&[&response_uri.to_string(), &mdoc_nonce]).unwrap(),
            )),
            nonce,
        };

        SessionTranscriptKeyed {
            device_engagement_bytes: None,
            ereader_key_bytes: None,
            handover: Handover::OID4VPHandover(handover.into()),
        }
        .into()
    }
}

pub type DeviceEngagementBytes = TaggedBytes<DeviceEngagement>;

/// Bytes/transcript of the first RP message with which the wallet and RP first established contact.
/// Differs per communication channel.
/// Through the [`SessionTranscript`], this is part of the [`DeviceAuthentication`] so it is signed
/// with each mdoc private key. This message is never sent but instead indenpendently computed by
/// the wallet and RP. If both sides do not agree on this message then mdoc verification fails.
///
/// The ISO standard(s) only uses serializations of this, and does not require the wallet or the RP
/// to ever deserialize this. (We have a custom deserializer for in test code, however.)
/// Serde's `untagged` enum representation ignores the enum variant name, and serializes instead
/// the contained data of the enum variant.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Handover {
    QRHandover,
    NFCHandover(CborSeq<NFCHandover>),
    SchemeHandoverBytes(TaggedBytes<ReaderEngagement>),
    OID4VPHandover(CborSeq<OID4VPHandover>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OID4VPHandover {
    /// Must be `SHA256(CBOR_encode([client_id, mdoc_nonce]))`
    pub client_id_hash: ByteBuf,
    /// Must be `SHA256(CBOR_encode([response_uri, mdoc_nonce]))`
    pub response_uri_hash: ByteBuf,
    pub nonce: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NFCHandover {
    pub handover_select_message: ByteBuf,
    pub handover_request_message: Option<ByteBuf>,
}

/// Describes available methods for the RP to connect to the holder.
pub type DeviceEngagement = CborIntMap<Engagement>;

/// Describes available methods for the holder to connect to the RP.
pub type ReaderEngagement = CborIntMap<Engagement>;

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Engagement {
    pub version: EngagementVersion,
    pub security: Option<Security>,
    pub connection_methods: Option<ConnectionMethods>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub origin_infos: Vec<OriginInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum EngagementVersion {
    #[serde(rename = "1.0")]
    V1_0,
}

/// Describes the kind and direction of the previously received protocol message.
/// Part of the [`DeviceAuthenticationBytes`] which are signed with the mdoc private key during disclosure.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct OriginInfo {
    pub cat: OriginInfoDirection,
    #[serde(flatten)]
    pub typ: OriginInfoType,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum OriginInfoDirection {
    Delivered = 0,
    Received = 1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginInfoType {
    Website(Url),
    OnDeviceQRCode,
    MessageData,
}

pub type Security = CborSeq<SecurityKeyed>;

/// The ephemeral public key used for establishing an E2E encrypted protocol channel.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SecurityKeyed {
    pub cipher_suite_identifier: CipherSuiteIdentifier,
    pub e_sender_key_bytes: ESenderKeyBytes,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Clone)]
#[repr(u8)]
pub enum CipherSuiteIdentifier {
    P256 = 1,
}

/// Describes the available connection methods. Called DeviceRetrievalMethods in ISO 18013-5
pub type ConnectionMethods = Vec<ConnectionMethod>;

/// Describes an available connection method. Called DeviceRetrievalMethod in ISO 18013-5
pub type ConnectionMethod = CborSeq<ConnectionMethodKeyed>;

/// Describes an available connection method.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConnectionMethodKeyed {
    pub typ: ConnectionMethodType,
    pub version: ConnectionMethodVersion,
    pub connection_options: CborSeq<RestApiOptionsKeyed>,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Clone)]
#[repr(u8)]
pub enum ConnectionMethodType {
    RestApi = 4,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Clone)]
#[repr(u8)]
pub enum ConnectionMethodVersion {
    RestApi = 1,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RestApiOptionsKeyed {
    pub uri: Url,
}

pub type ESenderKeyBytes = TaggedBytes<CoseKey>;

#[cfg(test)]
mod tests {
    use crate::{
        examples::{Example, EXAMPLE_DOC_TYPE},
        utils::serialization::{self, TaggedBytes},
    };

    use super::*;

    #[test]
    fn test_device_authentication_keyed_new() {
        let TaggedBytes(CborSeq(example_device_auth)) = DeviceAuthenticationBytes::example();
        let session_transcript = example_device_auth.session_transcript.into_owned();
        let device_auth = DeviceAuthenticationKeyed::new(EXAMPLE_DOC_TYPE, &session_transcript);

        assert_eq!(
            serialization::cbor_serialize(&TaggedBytes(CborSeq(device_auth))).unwrap(),
            DeviceAuthenticationBytes::example_bts()
        );
    }
}
