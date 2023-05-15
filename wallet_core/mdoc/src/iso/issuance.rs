use std::{borrow::Cow, fmt::Display};

use ciborium::value::Value;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use serde_with::skip_serializing_none;

use crate::serialization::{RequiredValue, RequiredValueTrait};

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "UPPERCASE")]
pub struct ServiceEngagement {
    pub id: RequiredValue<ServiceEngagementID>,
    pub url: Option<ServerUrl>,
    pub pc: Option<ProvisioningCode>,
    #[serde(rename = "Opt")]
    pub opt: Option<Options>,
}

#[derive(Debug, Clone)]
pub struct ServiceEngagementID;
impl RequiredValueTrait for ServiceEngagementID {
    type Type = Cow<'static, str>;
    const REQUIRED_VALUE: Self::Type = Cow::Borrowed("org.iso.23220-3-1.0");
}

pub type ProvisioningCode = String;
pub type ServerUrl = String;
pub type Options = IndexMap<OptionsKey, Value>;

/// Key for options in the [`Options`] map.
/// Options defined by ISO 23220-3 use non-negative integers as keys. All other options must use tstr
/// in the format [Reverse Domain].[Domain Specific Extension].[Key Name].
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum OptionsKey {
    Uint(u64),
    Tstr(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionId(ByteBuf);
impl From<ByteBuf> for SessionId {
    fn from(value: ByteBuf) -> Self {
        SessionId(value)
    }
}
impl From<Vec<u8>> for SessionId {
    fn from(value: Vec<u8>) -> Self {
        ByteBuf::from(value).into()
    }
}
impl Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(&self.0))
    }
}

pub const START_PROVISIONING_MSG_TYPE: &str = "StartProvisioning";

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename = "StartProvisioning")]
#[serde(tag = "messageType")]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub struct StartProvisioningMessage {
    pub provisioning_code: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename = "ReadyToProvision")]
#[serde(tag = "messageType")]
#[serde(rename_all = "camelCase")]
pub struct ReadyToProvisionMessage {
    pub e_session_id: SessionId,
}

// Session termination

pub const REQUEST_END_SESSION_MSG_TYPE: &str = "RequestEndSession";

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename = "RequestEndSession")]
#[serde(tag = "messageType")]
#[serde(rename_all = "camelCase")]
pub struct RequestEndSessionMessage {
    pub e_session_id: SessionId,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename = "EndSession")]
#[serde(tag = "messageType")]
#[serde(rename_all = "camelCase")]
pub struct EndSessionMessage {
    pub e_session_id: SessionId,
    pub reason: String, // "known values include success, failed, restart"
    pub delay: Option<u64>,
    #[serde(rename = "SED")]
    pub sed: Option<String>, // "e.g. new SED to be used by mdoc app to resume session"
}

#[cfg(test)]
mod tests {
    use crate::serialization::{cbor_deserialize, cbor_serialize};

    use super::*;

    #[test]
    fn test_options() {
        let map = Options::from([
            (OptionsKey::Tstr("hello".into()), Value::Text("world".into())),
            (OptionsKey::Uint(1), Value::Integer(42.into())),
        ]);

        // Explicitly assert CBOR structure of the serialized data
        assert_eq!(
            Value::serialized(&map).unwrap(),
            Value::Map(vec![
                (Value::Text("hello".into()), Value::Text("world".into())),
                (Value::Integer(1.into()), Value::Integer(42.into()))
            ])
        );

        // Check that we can deserialize to the same value
        let serialized = cbor_serialize(&map).unwrap();
        let deserialized: Options = cbor_deserialize(serialized.as_slice()).unwrap();
        assert_eq!(map, deserialized);
    }
}
