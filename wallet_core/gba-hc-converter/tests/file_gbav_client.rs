use gba_hc_converter::{
    gba::{
        client::{FileGbavClient, GbavClient},
        data::GbaResponse,
        error::Error,
    },
    haal_centraal::Element,
};

use crate::common::read_file;

mod common;

struct EmptyGbavClient {}

impl GbavClient for EmptyGbavClient {
    async fn vraag(&self, _bsn: &str) -> Result<GbaResponse, Error> {
        GbaResponse::new(&read_file("gba/empty-response.xml"))
    }
}

#[tokio::test]
async fn should_return_preloaded_xml() {
    let client = FileGbavClient::new("tests/resources/gba".into(), EmptyGbavClient {});
    let response = client.vraag("999991772").await.unwrap();
    assert_eq!(
        "Froukje",
        &response.categorievoorkomens[0]
            .elementen
            .get_mandatory(Element::Voornamen.code())
            .unwrap()
    );
}

#[tokio::test]
async fn should_return_empty() {
    let client = FileGbavClient::new("tests/resources/gba".into(), EmptyGbavClient {});
    let response = client.vraag("12345678").await.unwrap();
    assert!(response.categorievoorkomens.is_empty());
}
