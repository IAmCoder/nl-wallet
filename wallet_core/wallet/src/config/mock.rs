use std::sync::Arc;

use wallet_common::config::wallet_config::WalletConfiguration;

use crate::config::data::default_configuration;

use super::{
    ConfigCallback, ConfigurationError, ConfigurationRepository, ConfigurationUpdateState,
    ObservableConfigurationRepository, UpdateableConfigurationRepository,
};

pub struct LocalConfigurationRepository {
    config: Arc<WalletConfiguration>,
}

impl LocalConfigurationRepository {
    pub fn new(config: WalletConfiguration) -> Self {
        LocalConfigurationRepository {
            config: Arc::new(config),
        }
    }
}

impl Default for LocalConfigurationRepository {
    fn default() -> Self {
        Self::new(default_configuration())
    }
}

impl ConfigurationRepository for LocalConfigurationRepository {
    fn config(&self) -> Arc<WalletConfiguration> {
        Arc::clone(&self.config)
    }
}

impl UpdateableConfigurationRepository for LocalConfigurationRepository {
    async fn fetch(&self) -> Result<ConfigurationUpdateState, ConfigurationError> {
        Ok(ConfigurationUpdateState::Updated)
    }
}

impl ObservableConfigurationRepository for LocalConfigurationRepository {
    fn register_callback_on_update(&self, _callback: ConfigCallback) -> Option<ConfigCallback> {
        None
    }

    fn clear_callback(&self) -> Option<ConfigCallback> {
        None
    }
}
