use async_trait::async_trait;
use thiserror::Error;

use crate::domain::platform::Platform;
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
/// satisfy. Phase 1 ships stub adapters in
/// `infrastructure::connectors` that implement this trait and return
/// [`PlatformConnectorError::NotImplemented`] for every method — the point
/// is to fix the shape of the integration now so Phase 2+ swaps in a real
/// implementation without touching any caller.
#[async_trait]
pub trait PlatformConnector: Send + Sync {
    fn platform(&self) -> Platform;

    /// Begin (or complete) the OAuth/auth flow for an account on this
    /// platform. Returns an opaque external account id on success.
    async fn authenticate(&self) -> Result<String, PlatformConnectorError>;

    async fn disconnect(&self, external_account_id: &str) -> Result<(), PlatformConnectorError>;

    async fn validate_session(
        &self,
        external_account_id: &str,
    ) -> Result<bool, PlatformConnectorError>;

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
