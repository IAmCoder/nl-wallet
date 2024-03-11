use assert_matches::assert_matches;
use ciborium::Value;
use indexmap::IndexMap;
use reqwest::StatusCode;
use rstest::rstest;
use serial_test::serial;
use url::Url;

use nl_wallet_mdoc::{
    server_state::SessionToken,
    unsigned::Entry,
    verifier::{DisclosedAttributes, SessionType, StatusResponse},
    ItemsRequest,
};
use openid4vc::{oidc::MockOidcClient, token::TokenRequest};
use wallet::errors::DisclosureError;
use wallet_common::utils;
use wallet_server::verifier::{ReturnUrlTemplate, StartDisclosureRequest, StartDisclosureResponse};

use crate::common::*;

pub mod common;

async fn get_verifier_status(client: &reqwest::Client, session_url: Url) -> StatusResponse {
    let response = client.get(session_url).send().await.unwrap();

    assert!(response.status().is_success());

    response.json().await.unwrap()
}

#[rstest]
#[case(full_name(SessionType::SameDevice, None))]
#[case(full_name(SessionType::SameDevice, Some("http://localhost:3004/return".parse().unwrap())))]
#[case(full_name(SessionType::CrossDevice, None))]
#[case(full_name(SessionType::CrossDevice, Some("http://localhost:3004/return".parse().unwrap())))]
#[case(bsn(SessionType::SameDevice, None))]
#[case(multiple_cards(SessionType::SameDevice, None))]
#[case(duplicate_cards(SessionType::SameDevice, None))]
#[case(duplicate_attributes(SessionType::SameDevice, None))]
#[tokio::test]
#[serial]
async fn test_disclosure_usecases_ok(#[case] testcase: (StartDisclosureRequest, Vec<ExpectedAttribute>)) {
    let (start_request, expected_documents) = testcase;

    let digid_context = MockOidcClient::start_context();
    digid_context.expect().return_once(|_, _, _, _| {
        let mut session = MockOidcClient::default();

        session.expect_into_token_request().return_once(|_url| {
            Ok(TokenRequest {
                grant_type: openid4vc::token::TokenRequestGrantType::PreAuthorizedCode {
                    pre_authorized_code: utils::random_string(32).into(),
                },
                code_verifier: Some("my_code_verifier".to_string()),
                client_id: Some("my_client_id".to_string()),
                redirect_uri: Some("redirect://here".parse().unwrap()),
            })
        });

        Ok((session, Url::parse("http://localhost/").unwrap()))
    });

    let ws_settings = wallet_server_settings();

    let pin = "112233".to_string();
    let mut wallet = setup_wallet_and_env(
        config_server_settings(),
        wallet_provider_settings(),
        ws_settings.clone(),
    )
    .await;
    wallet = do_wallet_registration(wallet, pin.clone()).await;
    wallet = do_pid_issuance(wallet, pin.clone()).await;

    let client = reqwest::Client::new();

    let response = client
        .post(
            ws_settings
                .internal_url
                .join("disclosure/sessions")
                .expect("could not join url with endpoint"),
        )
        .json(&start_request)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let StartDisclosureResponse {
        session_url,
        engagement_url,
        mut disclosed_attributes_url,
    } = response.json::<StartDisclosureResponse>().await.unwrap();

    // after creating the session it should have status "Created"
    assert_matches!(
        get_verifier_status(&client, session_url.clone()).await,
        StatusResponse::Created
    );

    // disclosed attributes endpoint should return a response with code Bad Request when the status is not DONE
    let response = client.get(disclosed_attributes_url.clone()).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let proposal = wallet
        .start_disclosure(&engagement_url)
        .await
        .expect("Could not start disclosure");
    assert_eq!(proposal.documents.len(), expected_documents.len());

    // after the first wallet interaction it should have status "Waiting"
    assert_matches!(
        get_verifier_status(&client, session_url.clone()).await,
        StatusResponse::WaitingForResponse
    );

    // disclosed attributes endpoint should return a response with code Bad Request when the status is not DONE
    let response = client.get(disclosed_attributes_url.clone()).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let return_url = wallet
        .accept_disclosure(pin)
        .await
        .expect("Could not accept disclosure");

    // after disclosure it should have status "Done"
    assert_matches!(get_verifier_status(&client, session_url).await, StatusResponse::Done);

    // passing the transcript_hash this way only works reliably it is the only query paramater (which should be the case here)
    if let Some(url) = return_url {
        disclosed_attributes_url.set_query(url.query());
    }

    let response = client.get(disclosed_attributes_url).send().await.unwrap();
    let status = response.status();
    assert_eq!(status, StatusCode::OK);

    let disclosed_attributes = response.json::<DisclosedAttributes>().await.unwrap();

    for (doc_type, namespace, expected_entries) in expected_documents.into_iter() {
        // verify the disclosed attributes
        assert_eq!(
            disclosed_attributes.get(doc_type).unwrap().get(namespace).unwrap(),
            &expected_entries
        );
    }
}

#[tokio::test]
#[serial]
async fn test_disclosure_without_pid() {
    let digid_context = MockOidcClient::start_context();
    digid_context.expect().return_once(|_, _, _, _| {
        let session = MockOidcClient::default();
        Ok((session, Url::parse("http://localhost/").unwrap()))
    });

    let ws_settings = wallet_server_settings();

    let pin = "112233".to_string();
    let mut wallet = setup_wallet_and_env(
        config_server_settings(),
        wallet_provider_settings(),
        ws_settings.clone(),
    )
    .await;
    wallet = do_wallet_registration(wallet, pin.clone()).await;

    let client = reqwest::Client::new();

    let start_request = StartDisclosureRequest {
        usecase: "xyz_bank".to_owned(),
        session_type: SessionType::SameDevice,
        items_requests: vec![ItemsRequest {
            doc_type: "com.example.pid".to_owned(),
            request_info: None,
            name_spaces: IndexMap::from([(
                "com.example.pid".to_owned(),
                IndexMap::from_iter(
                    [("given_name", true), ("family_name", false)]
                        .iter()
                        .map(|(name, intent_to_retain)| (name.to_string(), *intent_to_retain)),
                ),
            )]),
        }]
        .into(),
        return_url_template: None,
    };
    let response = client
        .post(
            ws_settings
                .internal_url
                .join("disclosure/sessions")
                .expect("could not join url with endpoint"),
        )
        .json(&start_request)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // does it exist for the RP side of things?
    let StartDisclosureResponse {
        session_url,
        engagement_url,
        disclosed_attributes_url,
    } = response.json::<StartDisclosureResponse>().await.unwrap();

    assert_matches!(
        get_verifier_status(&client, session_url.clone()).await,
        StatusResponse::Created
    );

    let mut url = engagement_url.clone();
    url.set_query(Some("session_type=same_device"));

    let error = wallet
        .start_disclosure(&url)
        .await
        .expect_err("Should return error that attributes are not available");

    assert_matches!(
        get_verifier_status(&client, session_url.clone()).await,
        StatusResponse::WaitingForResponse
    );

    assert_matches!(
        error,
        DisclosureError::AttributesNotAvailable {
            missing_attributes: attrs,
            ..
        } if attrs
            .iter()
            .flat_map(|attr| attr.attributes.keys().map(|k| k.to_owned()).collect::<Vec<&str>>())
            .collect::<Vec<&str>>() == vec!["given_name", "family_name"]
    );

    wallet.cancel_disclosure().await.expect("Could not cancel disclosure");
    assert_matches!(
        get_verifier_status(&client, session_url.clone()).await,
        StatusResponse::Cancelled
    );

    let response = client.get(session_url).send().await.unwrap();
    let status = response.status();
    // a cancelled disclosure should have status 200
    assert_eq!(status, StatusCode::OK);

    let status = response.json::<StatusResponse>().await.unwrap();
    // and report the status as cancelled
    assert_matches!(status, StatusResponse::Cancelled);

    let response = client.get(disclosed_attributes_url).send().await.unwrap();
    // a cancelled disclosure does not result in any disclosed attributes
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_disclosure_not_found() {
    let settings = wallet_server_settings();
    start_wallet_server(settings.clone(), MockAttributeService(settings.issuer.certificates())).await;

    let client = reqwest::Client::new();
    // check if a freshly generated token returns a 404 on the status URL
    let response = client
        .get(
            settings
                .public_url
                .join(&format!("/{}/status", SessionToken::from("does_not_exist".to_owned())))
                .unwrap(),
        )
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // check if a freshly generated token returns a 404 on the wallet URL
    let response = client
        .post(
            settings
                .public_url
                .join(&format!("/{}", SessionToken::from("does_not_exist".to_owned())))
                .unwrap(),
        )
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // check if a freshly generated token returns a 404 on the disclosed_attributes URL
    let response = client
        .get(
            settings
                .internal_url
                .join(&format!(
                    "/{}/disclosed_attributes",
                    SessionToken::from("does_not_exist".to_owned())
                ))
                .unwrap(),
        )
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

trait StartDisclosure {
    fn start_disclosure_request(
        self,
        usecase: &str,
        return_url: Option<ReturnUrlTemplate>,
        session_type: SessionType,
    ) -> StartDisclosureRequest;
}

impl StartDisclosure for Vec<(&str, &str, &str)> {
    /// Generate StartDisclosureRequest, with a single [`ItemsRequest`] per attribute
    fn start_disclosure_request(
        self,
        usecase: &str,
        return_url: Option<ReturnUrlTemplate>,
        session_type: SessionType,
    ) -> StartDisclosureRequest {
        StartDisclosureRequest {
            usecase: usecase.to_string(),
            session_type,
            items_requests: self
                .iter()
                .map(move |(doc_type, namespace, attribute)| ItemsRequest {
                    doc_type: doc_type.to_string(),
                    name_spaces: IndexMap::from_iter(vec![(
                        namespace.to_string(),
                        IndexMap::from_iter(vec![(attribute.to_string(), true)].into_iter()),
                    )]),
                    request_info: None,
                })
                .collect::<Vec<_>>()
                .into(),
            // The setup script is hardcoded to include "http://localhost:3004/" in the `ReaderRegistration`
            // contained in the certificate, so we have to specify a return URL prefixed with that.
            return_url_template: return_url,
        }
    }
}

const PID: &str = "com.example.pid";
const ADDR: &str = "com.example.address";

fn requested_attribute(card: &'static str, name: &'static str) -> (&'static str, &'static str, &'static str) {
    (card, card, name)
}

fn expected_attributes(
    card: &'static str,
    attributes: Vec<(&'static str, impl Into<Value>)>,
) -> (&'static str, &'static str, Vec<Entry>) {
    (
        card,
        card,
        attributes
            .into_iter()
            .map(|(name, value)| Entry {
                name: name.into(),
                value: value.into(),
            })
            .collect(),
    )
}

type ExpectedAttribute = (&'static str, &'static str, Vec<Entry>);

fn full_name(
    session_type: SessionType,
    return_url: Option<ReturnUrlTemplate>,
) -> (StartDisclosureRequest, Vec<ExpectedAttribute>) {
    let requested_attributes = vec![
        requested_attribute(PID, "given_name"),
        requested_attribute(PID, "family_name"),
    ];

    let expected_attributes = vec![expected_attributes(
        PID,
        vec![("family_name", "De Bruijn"), ("given_name", "Willeke Liselotte")],
    )];

    (
        requested_attributes.start_disclosure_request("xyz_bank", return_url, session_type),
        expected_attributes,
    )
}

fn bsn(
    session_type: SessionType,
    return_url: Option<ReturnUrlTemplate>,
) -> (StartDisclosureRequest, Vec<ExpectedAttribute>) {
    let requested_attributes = vec![requested_attribute(PID, "bsn")];
    (
        requested_attributes.start_disclosure_request("bsn", return_url, session_type),
        vec![expected_attributes(PID, vec![("bsn", "999991772")])],
    )
}

fn multiple_cards(
    session_type: SessionType,
    return_url: Option<ReturnUrlTemplate>,
) -> (StartDisclosureRequest, Vec<ExpectedAttribute>) {
    let requested_attributes = vec![
        requested_attribute(PID, "given_name"),
        requested_attribute(ADDR, "resident_street"),
    ];
    (
        requested_attributes.start_disclosure_request("multiple_cards", return_url, session_type),
        vec![
            expected_attributes(PID, vec![("given_name", "Willeke Liselotte")]),
            expected_attributes(ADDR, vec![("resident_street", "Turfmarkt")]),
        ],
    )
}

fn duplicate_cards(
    session_type: SessionType,
    return_url: Option<ReturnUrlTemplate>,
) -> (StartDisclosureRequest, Vec<ExpectedAttribute>) {
    let requested_attributes = vec![
        requested_attribute(ADDR, "resident_street"),
        requested_attribute(ADDR, "resident_house_number"),
    ];
    (
        requested_attributes.start_disclosure_request("duplicate_cards", return_url, session_type),
        vec![expected_attributes(
            ADDR,
            vec![("resident_street", "Turfmarkt"), ("resident_house_number", "147")],
        )],
    )
}

fn duplicate_attributes(
    session_type: SessionType,
    return_url: Option<ReturnUrlTemplate>,
) -> (StartDisclosureRequest, Vec<ExpectedAttribute>) {
    let requested_attributes = vec![
        requested_attribute(PID, "given_name"),
        requested_attribute(PID, "given_name"),
    ];
    (
        requested_attributes.start_disclosure_request("duplicate_attributes", return_url, session_type),
        vec![expected_attributes(PID, vec![("given_name", "Willeke Liselotte")])],
    )
}
