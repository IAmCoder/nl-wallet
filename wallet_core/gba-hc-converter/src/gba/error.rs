use http::StatusCode;

use crate::gba::data::GbaResult;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("networking error: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("XML deserialization error: {0}")]
    XmlDeserialization(#[from] quick_xml::de::DeError),
    #[error("XML error: {0}")]
    Xml(#[from] quick_xml::Error),
    #[error("Categorie {0} is mandatory but missing")]
    MissingCategorie(u8),
    #[error("Element number {0} is mandatory but missing")]
    MissingElement(u16),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Unexpected response received")]
    UnexpectedResponse,
    #[error("Received error response: {0}")]
    GbaErrorResponse(GbaResult),
}

impl From<&Error> for StatusCode {
    fn from(value: &Error) -> Self {
        match value {
            Error::Transport(_) => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::PRECONDITION_FAILED,
        }
    }
}
