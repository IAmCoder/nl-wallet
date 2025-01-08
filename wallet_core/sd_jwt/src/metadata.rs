use base64::prelude::*;
use derive_more::Into;
use http::Uri;
use jsonschema::ValidationError;
use nutype::nutype;
use serde::Deserialize;
use serde::Serialize;
use serde_with::skip_serializing_none;

use wallet_common::utils::sha256;
use wallet_common::vec_at_least::VecNonEmpty;

#[derive(Debug, thiserror::Error)]
pub enum TypeMetadataError {
    #[error("json schema validation failed {0}")]
    JsonSchemaValidation(#[from] ValidationError<'static>),
    #[error("serialization failed {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("decoding failed {0}")]
    Decoding(#[from] base64::DecodeError),
    #[error("resource integrity check failed")]
    ResourceIntegrity,
}

/// Communicates that a type is optional in the specification it is derived from but implemented as mandatory due to
/// various reasons.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecOptionalImplRequired<T>(pub T);

pub const COSE_METADATA_HEADER_LABEL: &str = "vctm";
pub const COSE_METADATA_INTEGRITY_HEADER_LABEL: &str = "type_metadata_integrity";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedTypeMetadata {
    metadata_encoded: String,
    integrity: ResourceIntegrity,
}

impl ProtectedTypeMetadata {
    pub fn protect(metadata: &TypeMetadata) -> Result<Self, TypeMetadataError> {
        let bytes: Vec<u8> = serde_json::to_vec(&metadata)?;
        let metadata_encoded = BASE64_STANDARD.encode(bytes);
        let integrity = ResourceIntegrity::from_bytes(metadata_encoded.as_bytes());
        Ok(ProtectedTypeMetadata {
            metadata_encoded,
            integrity,
        })
    }

    pub fn verify_and_parse(&self) -> Result<TypeMetadata, TypeMetadataError> {
        let integrity = ResourceIntegrity::from_bytes(self.metadata_encoded.as_bytes());
        if self.integrity != integrity {
            return Err(TypeMetadataError::ResourceIntegrity);
        }

        let decoded = BASE64_STANDARD.decode(self.metadata_encoded.as_bytes())?;
        let metadata: TypeMetadata = serde_json::from_slice(&decoded)?;
        Ok(metadata)
    }

    pub fn metadata_encoded(&self) -> &str {
        &self.metadata_encoded
    }

    pub fn integrity(&self) -> &ResourceIntegrity {
        &self.integrity
    }
}

/// https://www.ietf.org/archive/id/draft-ietf-oauth-sd-jwt-vc-08.html#name-type-metadata-format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[skip_serializing_none]
pub struct TypeMetadata {
    /// A String or URI that uniquely identifies the type.
    pub vct: String,

    /// A human-readable name for the type, intended for developers reading the JSON document.
    pub name: Option<String>,

    /// A human-readable description for the type, intended for developers reading the JSON document.
    pub description: Option<String>,

    /// Another type that this type extends.
    #[serde(flatten)]
    pub extends: Option<MetadataExtendsOption>,

    /// An array of objects containing display information for the type.
    pub display: Vec<DisplayMetadata>,

    /// An array of objects containing claim information for the type.
    #[serde(default)]
    pub claims: Vec<ClaimMetadata>,

    /// A JSON Schema document describing the structure of the Verifiable Credential
    #[serde(flatten)]
    pub schema: SchemaOption,
}

#[derive(Debug, Clone, PartialEq, Eq, Into, Serialize, Deserialize)]
pub struct ResourceIntegrity(String);

impl ResourceIntegrity {
    const ALG_PREFIX: &'static str = "sha256";

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let sig = sha256(bytes);
        let integrity = format!("{}-{}", Self::ALG_PREFIX, BASE64_STANDARD.encode(sig));
        ResourceIntegrity(integrity)
    }
}

impl TryFrom<Vec<u8>> for TypeMetadata {
    type Error = serde_json::Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        serde_json::from_slice(&value)
    }
}

impl TryFrom<TypeMetadata> for Vec<u8> {
    type Error = serde_json::Error;

    fn try_from(value: TypeMetadata) -> Result<Self, Self::Error> {
        serde_json::to_vec(&value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MetadataExtendsOption {
    Uri {
        #[serde(flatten)]
        extends: MetadataExtends,
    },
    Identifier {
        extends: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataExtends {
    /// A URI of another type that this type extends.
    #[serde(with = "http_serde::uri")]
    pub extends: Uri,

    /// Validating the integrity of the extends field.
    #[serde(rename = "extends#integrity")]
    pub extends_integrity: SpecOptionalImplRequired<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SchemaOption {
    Embedded {
        /// An embedded JSON Schema document describing the structure of the Verifiable Credential.
        schema: JsonSchema,
    },
    Remote {
        /// A URL pointing to a JSON Schema document describing the structure of the Verifiable Credential.
        #[serde(with = "http_serde::uri")]
        schema_uri: Uri,
        /// Validating the integrity of the schema_uri field.
        #[serde(rename = "schema_uri#integrity")]
        schema_uri_integrity: SpecOptionalImplRequired<String>,
    },
}

#[nutype(validate(with = validate_json_schema, error = TypeMetadataError), derive(Debug, Clone, Serialize, Deserialize))]
pub struct JsonSchema(serde_json::Value);

fn validate_json_schema(schema: &serde_json::Value) -> Result<(), TypeMetadataError> {
    jsonschema::draft202012::meta::validate(schema).map_err(ValidationError::to_owned)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[skip_serializing_none]
pub struct DisplayMetadata {
    pub lang: String,
    pub name: String,
    pub description: Option<String>,
    pub rendering: Option<RenderingMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[skip_serializing_none]
pub enum RenderingMetadata {
    Simple {
        logo: Option<LogoMetadata>,
        background_color: Option<String>,
        text_color: Option<String>,
    },
    SvgTemplates,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoMetadata {
    #[serde(with = "http_serde::uri")]
    pub uri: Uri,

    #[serde(rename = "uri#integrity")]
    pub uri_integrity: SpecOptionalImplRequired<String>,

    pub alt_text: SpecOptionalImplRequired<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[skip_serializing_none]
pub struct ClaimMetadata {
    pub path: VecNonEmpty<ClaimPath>,
    #[serde(default)]
    pub display: Vec<ClaimDisplayMetadata>,
    #[serde(default)]
    pub sd: ClaimSelectiveDisclosureMetadata,
    pub svg_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum ClaimPath {
    SelectByKey(String),
    SelectAll,
    SelectByIndex(usize),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClaimSelectiveDisclosureMetadata {
    Always,
    #[default]
    Allowed,
    Never,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[skip_serializing_none]
pub struct ClaimDisplayMetadata {
    pub lang: String,
    pub label: String,
    pub description: Option<String>,
}

#[cfg(any(test, feature = "example_constructors"))]
pub mod mock {
    use serde_json::json;

    use wallet_common::utils::random_string;

    use crate::metadata::JsonSchema;
    use crate::metadata::SchemaOption;
    use crate::metadata::TypeMetadata;

    impl TypeMetadata {
        pub fn new_example() -> Self {
            Self {
                vct: random_string(16),
                name: Some(random_string(8)),
                description: None,
                extends: None,
                display: vec![],
                claims: vec![],
                schema: SchemaOption::Embedded {
                    schema: JsonSchema::try_new(json!({})).unwrap(),
                },
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::env;
    use std::path::PathBuf;

    use assert_matches::assert_matches;
    use serde_json::json;

    use crate::metadata::ClaimPath;
    use crate::metadata::MetadataExtendsOption;
    use crate::metadata::ProtectedTypeMetadata;
    use crate::metadata::SchemaOption;
    use crate::metadata::TypeMetadata;

    async fn read_and_parse_metadata(filename: &str) -> TypeMetadata {
        let base_path = env::var("CARGO_MANIFEST_DIR").map(PathBuf::from).unwrap();

        let metadata_file = tokio::fs::read(base_path.join("examples").join(filename))
            .await
            .unwrap();

        serde_json::from_slice(metadata_file.as_slice()).unwrap()
    }

    #[tokio::test]
    async fn test_deserialize() {
        let metadata = read_and_parse_metadata("example-metadata.json").await;

        assert_eq!(
            "https://sd_jwt_vc_metadata.example.com/example_credential",
            metadata.vct
        );
    }

    #[test]
    fn test_extends_with_identifier() {
        let metadata = serde_json::from_value::<TypeMetadata>(json!({
            "vct": "https://sd_jwt_vc_metadata.example.com/example_credential",
            "extends": "random_string",
            "display": [],
            "schema_uri": "https://sd_jwt_vc_metadata.example.com/",
            "schema_uri#integrity": "abc123",
        }))
        .unwrap();

        assert_matches!(metadata.extends, Some(MetadataExtendsOption::Identifier { .. }));
    }

    #[test]
    fn test_with_uri() {
        let metadata = serde_json::from_value::<TypeMetadata>(json!({
            "vct": "https://sd_jwt_vc_metadata.example.com/example_credential",
            "extends": "https://sd_jwt_vc_metadata.example.com/other_schema",
            "extends#integrity": "abc123",
            "display": [],
            "schema_uri": "https://sd_jwt_vc_metadata.example.com/",
            "schema_uri#integrity": "abc123",
        }))
        .unwrap();

        assert_matches!(metadata.extends, Some(MetadataExtendsOption::Uri { .. }));
        assert_matches!(metadata.schema, SchemaOption::Remote { .. });
    }

    #[test]
    fn test_embedded_schema_validation() {
        assert!(serde_json::from_value::<TypeMetadata>(json!({
            "vct": "https://sd_jwt_vc_metadata.example.com/example_credential",
            "extends": "https://sd_jwt_vc_metadata.example.com/other_schema",
            "extends#integrity": "abc123",
            "display": [],
            "schema": {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "flobject",
                "properties": {
                    "vct": {
                        "type": "string"
                    }
                }
            }
        }))
        .is_err());
    }

    #[tokio::test]
    async fn test_schema_validation_success() {
        let metadata = read_and_parse_metadata("example-metadata.json").await;

        let claims = json!({
          "vct":"https://credentials.example.com/identity_credential",
          "iss":"https://example.com/issuer",
          "nbf":1683000000,
          "exp":1883000000,
          "address":{
            "country":"DE"
          },
          "cnf":{
            "jwk":{
              "kty":"EC",
              "crv":"P-256",
              "x":"TCAER19Zvu3OHF4j4W4vfSVoHIP1ILilDls7vCeGemc",
              "y":"ZxjiWWbZMQGHVWKVQ4hbSIirsVfuecCE6t4jT9F2HZQ"
            }
          }
        });

        assert_eq!(
            vec![
                ClaimPath::SelectByKey(String::from("nationalities")),
                ClaimPath::SelectAll
            ],
            metadata.claims[5].path.clone().into_inner()
        );

        match metadata.schema {
            SchemaOption::Embedded { schema } => {
                assert!(jsonschema::draft202012::is_valid(&schema.into_inner(), &claims))
            }
            SchemaOption::Remote { .. } => {
                panic!("Remote schema option is not supported")
            }
        }
    }

    #[tokio::test]
    async fn test_schema_validation_failure() {
        let metadata = read_and_parse_metadata("example-metadata.json").await;

        let claims = json!({
          "address":{
            "country":123
          }
        });

        match metadata.schema {
            SchemaOption::Embedded { schema } => {
                assert!(jsonschema::draft202012::validate(&schema.into_inner(), &claims).is_err())
            }
            SchemaOption::Remote { .. } => {
                panic!("Remote schema option is not supported")
            }
        }
    }

    #[tokio::test]
    async fn test_sign_verify() {
        let metadata = read_and_parse_metadata("example-metadata.json").await;
        let signed = ProtectedTypeMetadata::protect(&metadata).unwrap();
        signed.verify_and_parse().unwrap();
    }
}
