use std::error::Error;

use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

use wallet_provider::{server, settings::Settings, wallet_config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let settings = Settings::new()?;
    let wallet_config = wallet_config::wallet_configuration()?;

    let builder = tracing_subscriber::fmt().with_env_filter(
        EnvFilter::builder()
            .with_default_directive(LevelFilter::INFO.into())
            .from_env_lossy(),
    );
    if settings.structured_logging {
        builder.json().init();
    } else {
        builder.init()
    }

    server::serve(settings, wallet_config).await?;

    Ok(())
}
