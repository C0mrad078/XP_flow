use async_trait::async_trait;
use tokio::sync::oneshot;

use crate::domain::auth_error::AuthError;
use crate::domain::oauth::AuthSession;
use crate::domain::platform::Platform;
use crate::domain::provider_identity::ConnectedIdentity;

/// Provider-specific authorization-flow strategy (section 4:
/// "`PlatformAuthProvider` -> `YouTubeAuthProvider`/`TikTokAuthProvider`/
/// `KwaiAuthProvider`"). One call drives the *entire* attempt — opening
/// the system browser, waiting for the user, and exchanging for real
/// credentials — because that is genuinely one indivisible unit of work
/// from the caller's perspective even though YouTube (a local loopback
/// listener talking directly to Google) and TikTok/Kwai (the Auth Broker,
/// polled for completion) implement it completely differently internally.
///
/// This is deliberately a *separate* trait from `PlatformConnector`
/// (`domain::ports::platform_connector`): starting a brand-new
/// authorization is a fundamentally different operation, with a different
/// transport per provider, from validating/refreshing/disconnecting an
/// *already-connected* account, which — once you have a `PlatformAccount`
/// — has the same shape for all three providers.
#[async_trait]
pub trait PlatformAuthProvider: Send + Sync {
    fn platform(&self) -> Platform;

    /// Drives one full authorization attempt to completion. `cancel` fires
    /// when the user clicks "Cancel" in the UI (section 45) — every
    /// implementation must observe it promptly and tear down whatever it
    /// opened (loopback listener, broker session) rather than leaving it
    /// running (section 116: "callback listeners left open").
    async fn authenticate(
        &self,
        session: AuthSession,
        cancel: oneshot::Receiver<()>,
    ) -> Result<ConnectedIdentity, AuthError>;
}
