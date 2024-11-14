use anyhow::Result;

use openid4vc::issuer::AttributeService;
use openid4vc::server_state::SessionStore;
use openid4vc::server_state::WteTracker;

use super::*;
use crate::issuer::create_issuance_router;
use crate::issuer::IssuerKeyRing;
use crate::settings::Settings;

pub async fn serve<A, IS, W>(attr_service: A, settings: Settings, issuance_sessions: IS, wte_tracker: W) -> Result<()>
where
    A: AttributeService + Send + Sync + 'static,
    IS: SessionStore<openid4vc::issuer::IssuanceData> + Send + Sync + 'static,
    W: WteTracker + Send + Sync + 'static,
{
    let log_requests = settings.log_requests;

    let private_keys: IssuerKeyRing<_> = settings.issuer.private_keys.try_into()?;
    let wallet_issuance_router = create_issuance_router(
        &settings.urls,
        private_keys,
        issuance_sessions,
        attr_service,
        settings.issuer.wallet_client_ids,
        settings.issuer.wte_issuer_pubkey.0,
        wte_tracker,
    )?;

    listen_wallet_only(
        settings.wallet_server,
        Router::new().nest("/issuance", wallet_issuance_router),
        log_requests,
    )
    .await
}
