mod credential_payload;

use serde::Deserialize;
use serde::Serialize;

use error_category::ErrorCategory;
use nl_wallet_mdoc::utils::auth::Organization;
use openid4vc::attributes::AttributeValue;
use sd_jwt::metadata::ClaimDisplayMetadata;
use sd_jwt::metadata::ClaimMetadata;
use sd_jwt::metadata::DisplayMetadata;

#[derive(Debug, thiserror::Error, ErrorCategory)]
pub enum AttestationError {
    #[error("error selecting attribute for claim: {0:?}")]
    #[category(pd)]
    AttributeNotFoundForClaim(ClaimMetadata),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attestation {
    pub identity: AttestationIdentity,
    pub attestation_type: String,
    pub display_metadata: Vec<DisplayMetadata>,
    pub issuer: Organization,
    pub attributes: Vec<AttestationAttribute>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttestationIdentity {
    Ephemeral,
    Fixed { id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttestationAttribute {
    pub key: String,
    pub labels: Vec<LocalizedString>,
    pub value: AttestationValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttestationValue {
    String { value: String },
    Boolean { value: bool },
    Number { value: i128 },
}

impl From<&AttributeValue> for AttestationValue {
    fn from(value: &AttributeValue) -> Self {
        match value {
            AttributeValue::Text(value) => AttestationValue::String { value: value.clone() },
            AttributeValue::Bool(value) => AttestationValue::Boolean { value: *value },
            AttributeValue::Number(value) => AttestationValue::Number { value: *value },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalizedString {
    pub language: String,
    pub value: String,
}

impl From<ClaimDisplayMetadata> for LocalizedString {
    fn from(value: ClaimDisplayMetadata) -> Self {
        Self {
            language: value.lang,
            value: value.label,
        }
    }
}
