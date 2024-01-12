use crate::common::*;

pub mod common;

#[tokio::test]
async fn test_wallet_registration_in_process() {
    let settings = wallet_provider_settings();
    let connection = database_connection(&settings).await;

    let wallet = setup_wallet_and_env(settings, wallet_server_settings(), pid_issuer_settings()).await;

    let before = wallet_user_count(&connection).await;
    do_wallet_registration(wallet, String::from("123344")).await;
    let after = wallet_user_count(&connection).await;

    assert_eq!(before + 1, after);
}
