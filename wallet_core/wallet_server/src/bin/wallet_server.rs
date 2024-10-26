use anyhow::Result;

use wallet_server::{
    pid::attributes::BrpPidAttributeService,
    server::{self, wallet_server_main},
    settings::Settings,
    store::{DatabaseConnection, SessionStoreVariant, WteTrackerVariant},
};

// Cannot use #[tokio::main], see: https://docs.sentry.io/platforms/rust/#async-main-function
fn main() -> Result<()> {
    wallet_server_main("wallet_server.toml", "wallet_server", async_main)
}

async fn async_main(settings: Settings) -> Result<()> {
    let storage_settings = &settings.storage;
    let db_connection = DatabaseConnection::try_new(storage_settings.url.clone()).await?;

    let disclosure_sessions = SessionStoreVariant::new(db_connection.clone(), storage_settings.into());
    let issuance_sessions = SessionStoreVariant::new(db_connection.clone(), storage_settings.into());
    let wte_tracker = WteTrackerVariant::new(db_connection);

    // This will block until the server shuts down.
    server::wallet_server::serve(
        BrpPidAttributeService::try_from(&settings.issuer)?,
        settings,
        disclosure_sessions,
        issuance_sessions,
        wte_tracker,
    )
    .await
}
