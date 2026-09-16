use async_trait::async_trait;
use thiserror::Error;

use crate::domain::auth_error::AuthError;
use crate::domain::platform::Platform;
use crate::domain::platform_account::PlatformAccount;
use crate::domain::provider_identity::{ConnectedIdentity, RefreshedCredentials};

#[derive(Debug, Error)]
pub enum PlatformConnectorError {
    #[error("{platform} connector is not yet implemented")]
    NotImplemented { platform: Platform },

    #[error("{platform} authentication required")]
    AuthRequired { platform: Platform },

    #[error("{platform} rate limit exceeded")]
    RateLimited { platform: Platform },

    #[error("{platform} request failed: {message}")]
    RequestFailed { platform: Platform, message: String },
}

/// Contract every social-platform integration (YouTube, TikTok, Kwai) must
/// satisfy for an *already-connected* account (section 35). Starting a
/// brand-new authorization is a separate concern — see
/// `domain::ports::platform_auth_provider::PlatformAuthProvider`; actually
/// publishing is a separate concern too — see
/// `domain::ports::platform_publisher::PlatformPublisher`, which replaced
/// this trait's original Phase 4 publish-related placeholder methods
/// (`publish_video`/`get_publication_status`/`fetch_metrics`/
/// `fetch_comments`) once Phase 5 needed a shape rich enough for
/// resumable/chunked/multi-step uploads. Metrics/comments will get their
/// own dedicated trait when a later phase actually implements them,
/// rather than living here unimplemented in the meantime.
#[async_trait]
pub trait PlatformConnector: Send + Sync {
    fn platform(&self) -> Platform;

    /// Re-fetches the provider's own identity for this account and
    /// confirms the stored credential still works — used both for the
    /// "Manage" screen's "last validated" and to detect a revoked/expired
    /// connection before the UI has to find out the hard way.
    async fn validate_connection(
        &self,
        account: &PlatformAccount,
    ) -> Result<ConnectedIdentity, AuthError>;

    /// Refreshes the account's access credential (section 37/38). Callers
    /// are responsible for the refresh-buffer/scheduling decision — this
    /// method always performs the refresh when called.
    async fn refresh_connection(
        &self,
        account: &PlatformAccount,
    ) -> Result<RefreshedCredentials, AuthError>;

    /// Revokes the connection provider-side where the provider supports
    /// it (section 56) — callers must not treat a revocation failure as
    /// fatal to the local disconnect, only log/report it.
    async fn disconnect(&self, account: &PlatformAccount) -> Result<(), AuthError>;

    async fn get_profile(&self, account: &PlatformAccount) -> Result<ConnectedIdentity, AuthError>;

    /// Persists a freshly-obtained local credential — YouTube only
    /// overrides this (its tokens live in the OS keychain); brokered
    /// providers' tokens never leave the Auth Broker, so the default no-op
    /// is correct for them (`PlatformAuthService` calls this generically
    /// right after a successful authorization, never knowing which
    /// provider actually needs it).
    async fn store_local_credential(
        &self,
        _account_id: uuid::Uuid,
        _credential: &crate::domain::provider_identity::LocalCredential,
    ) -> Result<(), AuthError> {
        Ok(())
    }
}
