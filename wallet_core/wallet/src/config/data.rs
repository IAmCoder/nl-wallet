use std::{str::FromStr, time::Duration};

use base64::prelude::*;
use once_cell::sync::Lazy;
use p256::{ecdsa::VerifyingKey, pkcs8::DecodePublicKey};
use reqwest::Certificate;
use url::Url;

use wallet_common::{
    config::wallet_config::{
        AccountServerConfiguration, BaseUrl, DisclosureConfiguration, LockTimeoutConfiguration,
        PidIssuanceConfiguration, WalletConfiguration, DEFAULT_UNIVERSAL_LINK_BASE,
    },
    trust_anchor::DerTrustAnchor,
};

// Each of these values can be overridden from environment variables at compile time
// when the `env_config` feature is enabled. Additionally, environment variables can
// be added to using a file named `.env` in root directory of this crate.
const WALLET_CONFIG_VERSION: &str = "1";

const CONFIG_SERVER_BASE_URL: &str = "http://localhost:3000/config/v1/";

const CONFIG_SERVER_TRUST_ANCHORS: &str = "";

const CONFIG_SERVER_SIGNING_PUBLIC_KEY: &str =
    "MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEW2zhAd/0VH7PzLdmAfDEmHpSWwbVRfr5H31fo2rQWtyU\
     oWZT/C5WSeVm5Ktp6nCwnOwhhJLLGb4K3LtUJeLKjA==";

const CONFIG_SERVER_UPDATE_FREQUENCY_IN_SEC: &str = "3600";
const WALLET_PROVIDER_BASE_URL: &str = "http://localhost:3000/api/v1/";

const CERTIFICATE_PUBLIC_KEY: &str = "MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEW2zhAd/0VH7PzLdmAfDEmHpSWwbVRfr5H31fo2rQWtyU\
                                      oWZT/C5WSeVm5Ktp6nCwnOwhhJLLGb4K3LtUJeLKjA==";

const DIGID_CLIENT_ID: &str = "";
const DIGID_URL: &str = "https://localhost:8006/";

const INSTRUCTION_RESULT_PUBLIC_KEY: &str = "MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEpQqynmHM6Iey1gqLPtTi4T9PflzCDpttyk\
                                             oP/iW47jE1Ra6txPJEPq4FVQdqQJEXcJ7i8TErVQ3KNB823StXnA==";

const PID_ISSUER_URL: &str = "http://localhost:3001/issuance/";

const MDOC_TRUST_ANCHORS: &str = "MIIBlTCCATqgAwIBAgIURlVkuYVVlqtiuecbOwVySS9jdFwwCgYIKoZIzj0EAwIwGTEXMBUGA1UEAwwO\
                                  Y2EuZXhhbXBsZS5jb20wHhcNMjMxMTI3MDc1NDMyWhcNMjQxMTI2MDc1NDMyWjAZMRcwFQYDVQQDDA5j\
                                  YS5leGFtcGxlLmNvbTBZMBMGByqGSM49AgEGCCqGSM49AwEHA0IABFPE9hj71n7dNJpV1lCBBExbCK1B\
                                  8KYu8q22Z5sPWzzzuUfRAM+K7NgsfQVprob1rR6U+pvemR1e992K8rua5gGjYDBeMB0GA1UdDgQWBBQv\
                                  7ArBe8g9qs+S0QVagvo1xhFd7TAfBgNVHSMEGDAWgBQv7ArBe8g9qs+S0QVagvo1xhFd7TAPBgNVHRMB\
                                  Af8EBTADAQH/MAsGA1UdDwQEAwIBBjAKBggqhkjOPQQDAgNJADBGAiEAuITZR9Rbj5zfzN39+PEymrnk\
                                  K8WVHjOID8jeajR4DC0CIQD9XnpbZLDYMCWqkVVeBMphwv8R3P1t3NSpXRQyLRIO2w==";

const RP_TRUST_ANCHORS: &str = "MIIBlDCCATqgAwIBAgIUMmfPjx+jkrbY6twjDTCNHtnoPB4wCgYIKoZIzj0EAwIwGTEXMBUGA1UEAwwO\
                                Y2EuZXhhbXBsZS5jb20wHhcNMjMxMTA3MTA1NDEzWhcNMjQxMTA2MTA1NDEzWjAZMRcwFQYDVQQDDA5j\
                                YS5leGFtcGxlLmNvbTBZMBMGByqGSM49AgEGCCqGSM49AwEHA0IABPD39ZBr4/cNp76DturjGWRtjjqU\
                                qQgt+Xny57i2xFYHRSogzdQYQbqdUOzEfBZeW3GkBjzmbCmug2zHr5B54UKjYDBeMB0GA1UdDgQWBBR4\
                                cYdOOiKhp1xTmK4ZW8JMG4CggzAfBgNVHSMEGDAWgBR4cYdOOiKhp1xTmK4ZW8JMG4CggzAPBgNVHRMB\
                                Af8EBTADAQH/MAsGA1UdDwQEAwIBBjAKBggqhkjOPQQDAgNIADBFAiBpJ/sEsPeTm8A2XYwRmu6NOkoL\
                                NqhPN569XKLTR6rVdwIhANOMtj2LwDUG2YcLkSBPSdhh/i/iCgTeuZQpOI8y+kBw";

const UNIVERSAL_LINK_BASE: &str = DEFAULT_UNIVERSAL_LINK_BASE;

macro_rules! config_default {
    ($name:ident) => {
        if cfg!(feature = "env_config") {
            // If the `env_config` feature is enabled, try to get the config default from
            // the environment variable of the same name, otherwise fall back to the constant.
            option_env!(stringify!($name)).unwrap_or($name)
        } else {
            $name
        }
    };
}

pub static UNIVERSAL_LINK_BASE_URL: Lazy<BaseUrl> = Lazy::new(|| {
    config_default!(UNIVERSAL_LINK_BASE)
        .parse::<BaseUrl>()
        .expect("Could not parse universal link base url")
});

pub fn init_universal_link_base_url() {
    Lazy::force(&UNIVERSAL_LINK_BASE_URL);
}

#[derive(Debug, Clone)]
pub struct ConfigServerConfiguration {
    pub base_url: Url,
    pub trust_anchors: Vec<Certificate>,
    pub signing_public_key: VerifyingKey,
    pub update_frequency: Duration,
}

impl Default for ConfigServerConfiguration {
    fn default() -> Self {
        Self {
            base_url: Url::parse(config_default!(CONFIG_SERVER_BASE_URL)).unwrap(),
            trust_anchors: config_default!(CONFIG_SERVER_TRUST_ANCHORS)
                .split('|')
                .map(|root_ca| reqwest::Certificate::from_der(&BASE64_STANDARD.decode(root_ca).unwrap()).unwrap())
                .collect(),
            signing_public_key: VerifyingKey::from_public_key_der(
                &BASE64_STANDARD
                    .decode(config_default!(CONFIG_SERVER_SIGNING_PUBLIC_KEY))
                    .unwrap(),
            )
            .unwrap(),
            update_frequency: Duration::from_secs(
                config_default!(CONFIG_SERVER_UPDATE_FREQUENCY_IN_SEC).parse().unwrap(),
            ),
        }
    }
}

fn parse_trust_anchors(source: &str) -> Vec<DerTrustAnchor> {
    source
        .split('|')
        .map(|anchor| serde_json::from_str(format!("\"{}\"", anchor).as_str()).expect("failed to parse trust anchor"))
        .collect()
}

pub fn default_configuration() -> WalletConfiguration {
    WalletConfiguration {
        version: u64::from_str(config_default!(WALLET_CONFIG_VERSION)).unwrap(),
        lock_timeouts: LockTimeoutConfiguration::default(),
        account_server: AccountServerConfiguration {
            base_url: Url::parse(config_default!(WALLET_PROVIDER_BASE_URL)).unwrap(),
            certificate_public_key: VerifyingKey::from_public_key_der(
                &BASE64_STANDARD.decode(config_default!(CERTIFICATE_PUBLIC_KEY)).unwrap(),
            )
            .unwrap()
            .into(),
            instruction_result_public_key: VerifyingKey::from_public_key_der(
                &BASE64_STANDARD
                    .decode(config_default!(INSTRUCTION_RESULT_PUBLIC_KEY))
                    .unwrap(),
            )
            .unwrap()
            .into(),
        },
        pid_issuance: PidIssuanceConfiguration {
            pid_issuer_url: Url::parse(config_default!(PID_ISSUER_URL)).unwrap(),
            digid_url: Url::parse(config_default!(DIGID_URL)).unwrap(),
            digid_client_id: String::from(config_default!(DIGID_CLIENT_ID)),
        },
        disclosure: DisclosureConfiguration {
            rp_trust_anchors: parse_trust_anchors(config_default!(RP_TRUST_ANCHORS)),
        },
        mdoc_trust_anchors: parse_trust_anchors(config_default!(MDOC_TRUST_ANCHORS)),
    }
}
