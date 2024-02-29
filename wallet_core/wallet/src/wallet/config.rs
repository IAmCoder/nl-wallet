use std::sync::Arc;

use wallet_common::config::wallet_config::WalletConfiguration;

use crate::config::ObservableConfigurationRepository;

use super::Wallet;

impl<CR, S, PEK, APC, DGS, IS, MDS> Wallet<CR, S, PEK, APC, DGS, IS, MDS>
where
    CR: ObservableConfigurationRepository,
{
    pub fn set_config_callback<F>(&self, callback: F)
    where
        F: Fn(Arc<WalletConfiguration>) + Send + Sync + 'static,
    {
        callback(self.config_repository.config());
        self.config_repository.register_callback_on_update(callback);
    }

    pub fn clear_config_callback(&self) {
        self.config_repository.clear_callback();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tokio::sync::Notify;

    use crate::config::default_configuration;

    use super::{super::test::WalletWithMocks, *};

    // Tests both setting and clearing the configuration callback.
    #[tokio::test]
    async fn test_wallet_set_clear_config_callback() {
        // Prepare an unregistered wallet.
        let wallet = WalletWithMocks::new_unregistered().await;

        // Wrap a `Vec<Configuration>` in both a `Mutex` and `Arc`,
        // so we can write to it from the closure.
        let configs = Arc::new(Mutex::new(Vec::<Arc<WalletConfiguration>>::with_capacity(1)));
        let callback_configs = Arc::clone(&configs);

        assert_eq!(Arc::strong_count(&configs), 2);

        let notifier = Arc::new(Notify::new());
        let callback_notifier = notifier.clone();

        // Set the configuration callback on the `Wallet`,
        // which should immediately be called exactly once.
        wallet.set_config_callback(move |config| {
            callback_configs.lock().unwrap().push(config);
            callback_notifier.notify_one();
        });

        // Wait for the callback to be completed.
        notifier.notified().await;

        // Infer that the closure is still alive by counting the `Arc` references.
        assert_eq!(Arc::strong_count(&configs), 2);

        // Test the contents of the `Vec<Configuration>`.
        {
            let configs = configs.lock().unwrap();

            assert_eq!(configs.len(), 1);
            assert_eq!(
                configs.first().unwrap().account_server.base_url,
                default_configuration().account_server.base_url
            );
        }

        // Clear the configuration callback on the `Wallet.`
        wallet.clear_config_callback();

        assert_eq!(Arc::strong_count(&configs), 1);
    }
}
