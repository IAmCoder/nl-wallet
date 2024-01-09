use async_trait::async_trait;
use futures::future::TryFutureExt;
use serde::{de::DeserializeOwned, Serialize};
use url::Url;
use x509_parser::nom::AsBytes;

use crate::{
    utils::serialization::{cbor_deserialize, cbor_serialize, CborError},
    Error,
};

use super::HolderError;

#[derive(Debug, thiserror::Error)]
pub enum HttpClientError {
    #[error("CBOR error: {0}")]
    Cbor(#[from] CborError),
    #[error("HTTP request error: {0}")]
    Request(#[from] reqwest::Error),
}

impl From<HttpClientError> for Error {
    fn from(source: HttpClientError) -> Self {
        Self::Holder(HolderError::from(source))
    }
}

pub type HttpClientResult<R> = std::result::Result<R, HttpClientError>;

#[async_trait]
pub trait HttpClient {
    async fn post<R, V>(&self, url: &Url, val: &V) -> HttpClientResult<R>
    where
        V: Serialize + Sync,
        R: DeserializeOwned;
}

/// Send and receive CBOR-encoded messages over HTTP using a [`reqwest::Client`].
pub struct CborHttpClient(pub reqwest::Client);

#[async_trait]
impl HttpClient for CborHttpClient {
    async fn post<R, V>(&self, url: &Url, val: &V) -> HttpClientResult<R>
    where
        V: Serialize + Sync,
        R: DeserializeOwned,
    {
        let bytes = cbor_serialize(val)?;
        let response_bytes = self
            .0
            .post(url.clone())
            .body(bytes)
            .send()
            .and_then(|response| async { response.error_for_status()?.bytes().await })
            .await
            .map_err(HttpClientError::Request)?;
        let response = cbor_deserialize(response_bytes.as_bytes())?;
        Ok(response)
    }
}
