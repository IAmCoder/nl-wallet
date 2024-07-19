use tracing::{info, instrument, warn};

use error_category::{sentry_capture_error, ErrorCategory};
use wallet_common::keys::StoredByIdentifier;

use crate::storage::Storage;

use super::Wallet;

#[derive(Debug, thiserror::Error, ErrorCategory)]
pub enum ResetError {
    #[error("wallet is not registered")]
    #[category(expected)]
    NotRegistered,
}

type ResetResult<T> = std::result::Result<T, ResetError>;

impl<CR, S, PEK, APC, DS, IS, MDS> Wallet<CR, S, PEK, APC, DS, IS, MDS>
where
    S: Storage,
    PEK: StoredByIdentifier,
{
    pub(super) async fn reset_to_initial_state(&mut self) -> bool {
        // Only reset if we actually have a registration.
        if let Some(registration) = self.registration.take() {
            info!("Resetting wallet to inital state and wiping all local data");

            // Clear the database and its encryption key.
            self.storage.get_mut().clear().await;

            // Delete the hardware private key, log any potential error.
            if let Err(error) = registration.hw_privkey.delete().await {
                warn!("Could not delete hardware private key: {0}", error);
            };

            self.issuance_session.take();
            self.disclosure_session.take();

            // The wallet should be locked in its initial state.
            self.lock.lock();

            return true;
        }

        false
    }

    #[instrument(skip_all)]
    #[sentry_capture_error]
    pub async fn reset(&mut self) -> ResetResult<()> {
        info!("Resetting of wallet requested");

        // Note that this method can be called even if the Wallet is locked!

        info!("Checking if registered");
        if !self.reset_to_initial_state().await {
            return Err(ResetError::NotRegistered);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;

    use openid4vc::mock::MockIssuanceSession;
    use wallet_common::keys::software::SoftwareEcdsaKey;

    use crate::{disclosure::MockMdocDisclosureSession, storage::StorageState};

    use super::{
        super::{issuance::PidIssuanceSession, registration, test::WalletWithMocks},
        *,
    };

    #[tokio::test]
    async fn test_wallet_reset() {
        // Test resetting a registered and unlocked Wallet.
        let mut wallet = WalletWithMocks::new_registered_and_unlocked().await;

        assert!(SoftwareEcdsaKey::identifier_exists(
            registration::wallet_key_id().as_ref()
        ));

        // Check that the hardware key exists.
        wallet
            .reset()
            .await
            .expect("resetting the Wallet should have succeeded");

        // The database should now be uninitialized, the hardware key should
        // be gone and the `Wallet` should be both unregistered and locked.
        assert!(wallet.registration.is_none());
        assert_matches!(
            wallet.storage.get_mut().state().await.unwrap(),
            StorageState::Uninitialized
        );
        assert!(!SoftwareEcdsaKey::identifier_exists(
            registration::wallet_key_id().as_ref()
        ));
        assert!(wallet.is_locked());
    }

    #[tokio::test]
    async fn test_wallet_reset_full() {
        // Create the impossible Wallet that is doing everything at once and reset it.
        let mut wallet = WalletWithMocks::new_registered_and_unlocked().await;
        wallet.issuance_session = PidIssuanceSession::Openid4vci(MockIssuanceSession::default()).into();
        wallet.disclosure_session = MockMdocDisclosureSession::default().into();

        // Check that the hardware key exists.
        assert!(SoftwareEcdsaKey::identifier_exists(
            registration::wallet_key_id().as_ref()
        ));

        wallet
            .reset()
            .await
            .expect("resetting the Wallet should have succeeded");

        // The wallet should now be totally cleared, even though the PidIssuerClient returned an error.
        assert!(wallet.registration.is_none());
        assert_matches!(
            wallet.storage.get_mut().state().await.unwrap(),
            StorageState::Uninitialized
        );
        assert!(!SoftwareEcdsaKey::identifier_exists(
            registration::wallet_key_id().as_ref()
        ));
        assert!(wallet.issuance_session.is_none());
        assert!(wallet.disclosure_session.is_none());
        assert!(wallet.is_locked());
    }

    #[tokio::test]
    async fn test_wallet_reset_error_not_registered() {
        let mut wallet = WalletWithMocks::new_unregistered().await;

        // Attempting to reset an unregistered Wallet should result in an error.
        let error = wallet
            .reset()
            .await
            .expect_err("resetting the Wallet should have resulted in an error");

        assert_matches!(error, ResetError::NotRegistered);
    }
}
