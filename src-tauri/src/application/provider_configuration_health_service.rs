use serde::Serialize;

use crate::domain::platform::Platform;
use crate::infrastructure::auth::broker_client::BrokerClient;

/// One provider's connect-readiness, as the backend actually knows it —
/// the single signal every Connect Account surface (Settings →
/// Integrations, the Channels inline picker, anything added later) reads
/// instead of each independently guessing from partial config. Never
/// used to decide whether a platform is *shown*: all three platforms are
/// always shown; this only decides what the Connect action does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderConfigurationStatus {
    Ready,
    ConfigurationRequired,
    BrokerUnavailable,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderConfigurationHealth {
    pub platform: Platform,
    pub available: bool,
    pub status: ProviderConfigurationStatus,
    /// Non-secret configuration names only (e.g. `"YOUTUBE_CLIENT_ID"`) —
    /// never a secret's name or value. See module docs on each `*Config`
    /// type for the public/confidential split this reads from.
    pub missing_configuration: Vec<String>,
    pub user_message: Option<String>,
}

/// Computes `ProviderConfigurationHealth` for all three platforms from
/// the exact same facts `lib.rs` already used to decide between a real
/// auth provider/connector and a `Stub*` at startup (section 6: one
/// backend-authoritative model, not several components each re-deriving
/// it). The static facts are captured once at construction; broker
/// reachability is re-checked live on every query, since the broker can
/// go up or down after the desktop app has already started.
///
/// YouTube carries no configuration fact here at all: `YouTubeAuthConfig`
/// always resolves (a built-in OAuth client id ships with the app), so
/// YouTube is unconditionally `Ready`.
pub struct ProviderConfigurationHealthService {
    tiktok_configured: bool,
    broker_client: Option<BrokerClient>,
}

impl ProviderConfigurationHealthService {
    pub fn new(tiktok_configured: bool, broker_client: Option<BrokerClient>) -> Self {
        Self {
            tiktok_configured,
            broker_client,
        }
    }

    pub async fn check_all(&self) -> Vec<ProviderConfigurationHealth> {
        let broker_reachable = self.broker_reachable().await;
        vec![
            Self::youtube_health(),
            self.brokered_health(Platform::TikTok, self.tiktok_configured, broker_reachable),
            self.brokered_health(Platform::Kwai, true, broker_reachable),
        ]
    }

    /// `None` when no broker client exists at all (not configured, or
    /// configured but rejected by `AuthBrokerConfig::is_secure_enough`)
    /// — distinct from `Some(false)`, a broker that exists but didn't
    /// answer `/v1/health` just now.
    async fn broker_reachable(&self) -> Option<bool> {
        match &self.broker_client {
            None => None,
            Some(client) => Some(client.health_check().await.is_ok()),
        }
    }

    fn youtube_health() -> ProviderConfigurationHealth {
        ProviderConfigurationHealth {
            platform: Platform::YouTube,
            available: true,
            status: ProviderConfigurationStatus::Ready,
            missing_configuration: Vec::new(),
            user_message: None,
        }
    }

    /// Shared shape for TikTok and Kwai: both connect through the Auth
    /// Broker, so both go through configuration-required (own client
    /// config missing) → configuration-required (broker not configured
    /// or not trusted) → broker-unavailable (broker configured but not
    /// reachable right now) → ready, in that order. TikTok additionally
    /// needs its own `TIKTOK_CLIENT_KEY`; Kwai's connect flow does not
    /// (`KWAI_APP_ID` only gates publishing, not connecting — see
    /// `KwaiPublishConfig`).
    fn brokered_health(
        &self,
        platform: Platform,
        own_config_present: bool,
        broker_reachable: Option<bool>,
    ) -> ProviderConfigurationHealth {
        if !own_config_present {
            let missing = match platform {
                Platform::TikTok => "TIKTOK_CLIENT_KEY",
                Platform::Kwai => "KWAI_APP_ID",
                Platform::YouTube => "YOUTUBE_CLIENT_ID",
            };
            return ProviderConfigurationHealth {
                platform,
                available: false,
                status: ProviderConfigurationStatus::ConfigurationRequired,
                missing_configuration: vec![missing.to_string()],
                user_message: Some(format!(
                    "{} developer configuration is missing. Set {missing} to enable connecting a {} account.",
                    platform.display_name(),
                    platform.display_name(),
                )),
            };
        }

        match broker_reachable {
            None => ProviderConfigurationHealth {
                platform,
                available: false,
                status: ProviderConfigurationStatus::ConfigurationRequired,
                missing_configuration: vec!["XPFLOW_AUTH_BROKER_URL".to_string()],
                user_message: Some(
                    "The authentication service is not configured. Set XPFLOW_AUTH_BROKER_URL to a trusted (HTTPS or loopback) address.".to_string(),
                ),
            },
            Some(false) => ProviderConfigurationHealth {
                platform,
                available: false,
                status: ProviderConfigurationStatus::BrokerUnavailable,
                missing_configuration: Vec::new(),
                user_message: Some(
                    "Authentication service unavailable. It may be starting up or temporarily down.".to_string(),
                ),
            },
            Some(true) => ProviderConfigurationHealth {
                platform,
                available: true,
                status: ProviderConfigurationStatus::Ready,
                missing_configuration: Vec::new(),
                user_message: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn broker_config_loopback() -> crate::infrastructure::auth::broker_config::AuthBrokerConfig {
        crate::infrastructure::auth::broker_config::AuthBrokerConfig::Development {
            base_url: "http://127.0.0.1:8787".to_string(),
        }
    }

    #[tokio::test]
    async fn youtube_is_always_ready_with_no_configuration_facts_needed() {
        // No env, no broker — YouTube's built-in OAuth client id means it
        // never depends on either.
        let service = ProviderConfigurationHealthService::new(false, None);
        let health = service.check_all().await;
        let youtube = health
            .iter()
            .find(|h| h.platform == Platform::YouTube)
            .unwrap();
        assert_eq!(youtube.status, ProviderConfigurationStatus::Ready);
        assert!(youtube.available);
        assert!(youtube.missing_configuration.is_empty());
    }

    #[tokio::test]
    async fn all_three_platforms_are_always_present_even_when_tiktok_kwai_are_unconfigured() {
        let service = ProviderConfigurationHealthService::new(false, None);
        let health = service.check_all().await;
        assert_eq!(health.len(), 3);
        assert!(health.iter().any(|h| h.platform == Platform::YouTube));
        assert!(health.iter().any(|h| h.platform == Platform::TikTok));
        assert!(health.iter().any(|h| h.platform == Platform::Kwai));
        assert_eq!(
            health
                .iter()
                .find(|h| h.platform == Platform::YouTube)
                .unwrap()
                .status,
            ProviderConfigurationStatus::Ready
        );
        assert!(health
            .iter()
            .filter(|h| h.platform != Platform::YouTube)
            .all(|h| h.status == ProviderConfigurationStatus::ConfigurationRequired));
    }

    #[tokio::test]
    async fn tiktok_missing_client_key_reports_that_specific_env_var() {
        let service = ProviderConfigurationHealthService::new(false, None);
        let health = service.check_all().await;
        let tiktok = health
            .iter()
            .find(|h| h.platform == Platform::TikTok)
            .unwrap();
        assert_eq!(
            tiktok.status,
            ProviderConfigurationStatus::ConfigurationRequired
        );
        assert_eq!(tiktok.missing_configuration, vec!["TIKTOK_CLIENT_KEY"]);
    }

    #[tokio::test]
    async fn kwai_does_not_require_kwai_app_id_to_connect() {
        // KWAI_APP_ID only gates publishing (KwaiPublishConfig), not the
        // connect/auth flow — with no broker configured at all, Kwai's
        // gap is reported as the broker, never a Kwai-specific client id.
        let service = ProviderConfigurationHealthService::new(true, None);
        let health = service.check_all().await;
        let kwai = health
            .iter()
            .find(|h| h.platform == Platform::Kwai)
            .unwrap();
        assert_eq!(
            kwai.status,
            ProviderConfigurationStatus::ConfigurationRequired
        );
        assert_eq!(kwai.missing_configuration, vec!["XPFLOW_AUTH_BROKER_URL"]);
    }

    #[tokio::test]
    async fn broker_configured_but_unreachable_is_broker_unavailable_not_hidden() {
        let config = broker_config_loopback();
        // Port 8787 with nothing listening on it in the test environment —
        // exercises the real transport-error path through `health_check`.
        let client = BrokerClient::new(&config);
        let service = ProviderConfigurationHealthService::new(true, Some(client));
        let health = service.check_all().await;
        let tiktok = health
            .iter()
            .find(|h| h.platform == Platform::TikTok)
            .unwrap();
        let kwai = health
            .iter()
            .find(|h| h.platform == Platform::Kwai)
            .unwrap();
        for provider in [tiktok, kwai] {
            assert_eq!(
                provider.status,
                ProviderConfigurationStatus::BrokerUnavailable
            );
            assert!(!provider.available);
            assert!(provider.user_message.is_some());
        }
    }
}
