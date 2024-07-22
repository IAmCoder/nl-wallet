use std::{collections::HashMap, result::Result as StdResult, sync::Arc};

use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use base64::prelude::*;
use memory_serve::{load_assets, CacheControl, MemoryServe};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::warn;
use url::Url;

use nl_wallet_mdoc::{server_state::SessionToken, verifier::DisclosedAttributes};
use wallet_common::{config::wallet_config::BaseUrl, utils::sha256};

use crate::{
    askama_axum,
    client::WalletServerClient,
    settings::{Origin, ReturnUrlMode, Settings, Usecase, WalletWeb},
};

#[derive(Debug)]
pub struct Error(anyhow::Error);

impl From<anyhow::Error> for Error {
    fn from(error: anyhow::Error) -> Self {
        Self(error)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        warn!("error result: {:?}", self);
        (StatusCode::INTERNAL_SERVER_ERROR, format!("{}", self.0)).into_response()
    }
}

type Result<T> = StdResult<T, Error>;

const RETURN_URL_SEGMENT: &str = "return";

struct ApplicationState {
    client: WalletServerClient,
    public_wallet_server_url: BaseUrl,
    public_url: BaseUrl,
    usecases: HashMap<String, Usecase>,
    wallet_web: WalletWeb,
}

fn cors_layer(allow_origins: Vec<Origin>) -> Option<CorsLayer> {
    if allow_origins.is_empty() {
        return None;
    }

    let layer = CorsLayer::new()
        .allow_origin(
            allow_origins
                .into_iter()
                .map(|url| {
                    url.try_into()
                        .expect("cross_origin base_url should be parseable to header value")
                })
                .collect::<Vec<_>>(),
        )
        .allow_headers(Any)
        .allow_methods([Method::GET, Method::POST]);

    Some(layer)
}

pub fn create_router(settings: Settings) -> Router {
    let application_state = Arc::new(ApplicationState {
        client: WalletServerClient::new(settings.internal_wallet_server_url.clone()),
        public_wallet_server_url: settings.public_wallet_server_url,
        public_url: settings.public_url,
        usecases: settings.usecases,
        wallet_web: settings.wallet_web,
    });

    let mut app = Router::new()
        .route("/sessions", post(create_session))
        .route("/:usecase/", get(usecase))
        .route(&format!("/:usecase/{}", RETURN_URL_SEGMENT), get(disclosed_attributes))
        .fallback_service(
            MemoryServe::new(load_assets!("assets"))
                .cache_control(CacheControl::NoCache)
                .into_router()
                .into_service(),
        )
        .with_state(application_state)
        .layer(TraceLayer::new_for_http());

    if let Some(cors) = cors_layer(settings.allow_origins) {
        app = app.layer(cors)
    }

    app
}

#[derive(Deserialize, Serialize)]
struct SessionOptions {
    usecase: String,
}

#[derive(Serialize)]
struct SessionResponse {
    status_url: Url,
    session_token: SessionToken,
}

#[derive(Template, Serialize)]
#[template(path = "disclosed/attributes.askama", escape = "html", ext = "html")]
struct DisclosureTemplate<'a> {
    usecase: &'a str,
    attributes: DisclosedAttributes,
}

#[derive(Template, Serialize)]
#[template(path = "usecase/usecase.askama", escape = "html", ext = "html")]
struct UsecaseTemplate<'a> {
    usecase: &'a str,
    usecase_js_sha256: &'a str,
    wallet_web_filename: &'a str,
    wallet_web_sha256: &'a str,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DisclosedAttributesParams {
    pub nonce: Option<String>,
    pub session_token: SessionToken,
}

async fn create_session(
    State(state): State<Arc<ApplicationState>>,
    Json(options): Json<SessionOptions>,
) -> Result<Json<SessionResponse>> {
    let usecase = state
        .usecases
        .get(&options.usecase)
        .ok_or(anyhow::Error::msg("usecase not found"))?;

    let session_token = state
        .client
        .start(
            options.usecase.clone(),
            usecase.items_requests.clone(),
            if usecase.return_url == ReturnUrlMode::None {
                None
            } else {
                Some(
                    format!(
                        "{}/{}?session_token={{session_token}}",
                        state.public_url.join(&options.usecase),
                        RETURN_URL_SEGMENT
                    )
                    .parse()
                    .expect("should always be a valid ReturnUrlTemplate"),
                )
            },
        )
        .await?;

    let result = SessionResponse {
        status_url: state
            .public_wallet_server_url
            .join(&format!("disclosure/sessions/{session_token}")),
        session_token,
    };
    Ok(result.into())
}

static USECASE_JS_SHA256: Lazy<String> =
    Lazy::new(|| BASE64_STANDARD.encode(sha256(include_bytes!("../assets/usecase.js"))));

async fn usecase(State(state): State<Arc<ApplicationState>>, Path(usecase): Path<String>) -> Result<Response> {
    if !state.usecases.contains_key(&usecase) {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }

    let result = UsecaseTemplate {
        usecase: &usecase,
        usecase_js_sha256: &USECASE_JS_SHA256,
        wallet_web_filename: &state.wallet_web.filename.to_string_lossy(),
        wallet_web_sha256: &state.wallet_web.sha256,
    };

    Ok(askama_axum::into_response(&result))
}

async fn disclosed_attributes(
    State(state): State<Arc<ApplicationState>>,
    Path(usecase): Path<String>,
    Query(params): Query<DisclosedAttributesParams>,
) -> Result<Response> {
    if !state.usecases.contains_key(&usecase) {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }

    let attributes = state
        .client
        .disclosed_attributes(params.session_token, params.nonce)
        .await?;

    let result = DisclosureTemplate {
        usecase: &usecase,
        attributes,
    };

    Ok(askama_axum::into_response(&result))
}

mod filters {
    use nl_wallet_mdoc::verifier::DisclosedAttributes;

    pub fn attribute(attributes: &DisclosedAttributes, name: &str) -> ::askama::Result<String> {
        for doctype in attributes {
            for namespace in doctype.1.attributes.iter() {
                for attribute in namespace.1 {
                    if attribute.name == name {
                        return Ok(attribute.value.as_text().unwrap().to_owned());
                    }
                }
            }
        }

        Ok(format!("attribute '{name}' cannot be found"))
    }
}
