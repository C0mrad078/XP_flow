use async_trait::async_trait;
use thiserror::Error;

use crate::domain::auth_error::AuthError;
use crate::domain::platform::Platform;
use crate::domain::platform_account::PlatformAccount;
use crate::domain::provider_identity::{ConnectedIdentity, RefreshedCredentials};
use crate::domain::publication::Publication;

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
/// `domain::ports::platform_auth_provider::PlatformAuthProvider`.
///
/// Phase 1-3 shipped stub adapters that returned `NotImplemented` for
/// everything; Phase 4 implements the account-lifecycle methods for real
/// per provider. The publishing-related methods stay defined-but-
/// unimplemented until Phase 5 (section 112) — fixing their shape now is
/// what lets Phase 5 add a real implementation without touching any
/// caller.
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

    // --- Phase 5 seam: defined now, deliberately unimplemented (section 112) ---
    async fn publish_video(
        &self,
        publication: &Publication,
    ) -> Result<String, PlatformConnectorError>;
    async fn get_publication_status(
        &self,
        remote_id: &str,
    ) -> Result<String, PlatformConnectorError>;
    async fn fetch_metrics(
        &self,
        remote_id: &str,
    ) -> Result<serde_json::Value, PlatformConnectorError>;
    async fn fetch_comments(
        &self,
        remote_id: &str,
    ) -> Result<serde_json::Value, PlatformConnectorError>;
}
