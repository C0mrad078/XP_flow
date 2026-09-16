//! Real `PlatformConnector`/`PlatformAuthProvider` implementations per
//! platform (section 35/36), plus `StubConnector` — now used specifically
//! as the graceful-degradation fallback for a platform whose developer
//! configuration is missing (section 65: "do not crash the whole
//! application because one provider is not configured").

pub mod kwai;
pub mod tiktok;
pub mod youtube;

use async_trait::async_trait;
use tokio::sync::oneshot;

use crate::domain::auth_error::AuthError;
use crate::domain::oauth::AuthSession;
use crate::domain::platform::Platform;
use crate::domain::platform_account::PlatformAccount;
use crate::domain::ports::platform_auth_provider::PlatformAuthProvider;
use crate::domain::ports::platform_connector::PlatformConnector;
use crate::domain::provider_identity::ConnectedIdentity;

/// Wired in for a platform whose non-secret developer configuration
/// (client id/key, or the Auth Broker itself) is unavailable at startup —
/// every account-lifecycle call fails with a clear, typed
/// `ProviderNotConfigured` rather than the app refusing to start or a
/// panic reaching the user.
pub struct StubConnector {
    platform: Platform,
    reason: String,
}

impl StubConnector {
    pub fn new(platform: Platform, reason: impl Into<String>) -> Self {
        Self {
            platform,
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl PlatformConnector for StubConnector {
    fn platform(&self) -> Platform {
        self.platform
    }

    async fn validate_connection(
        &self,
        _account: &PlatformAccount,
    ) -> Result<ConnectedIdentity, AuthError> {
        Err(AuthError::ProviderNotConfigured {
            detail: self.reason.clone(),
        })
    }

    async fn refresh_connection(
        &self,
        _account: &PlatformAccount,
    ) -> Result<crate::domain::provider_identity::RefreshedCredentials, AuthError> {
        Err(AuthError::ProviderNotConfigured {
            detail: self.reason.clone(),
        })
    }

    async fn disconnect(&self, _account: &PlatformAccount) -> Result<(), AuthError> {
        Err(AuthError::ProviderNotConfigured {
            detail: self.reason.clone(),
        })
    }

    async fn get_profile(
        &self,
        _account: &PlatformAccount,
    ) -> Result<ConnectedIdentity, AuthError> {
        Err(AuthError::ProviderNotConfigured {
            detail: self.reason.clone(),
        })
    }
}

/// The `PlatformAuthProvider` counterpart to `StubConnector` — wired in
/// for a platform whose configuration is missing so "Connect" fails with
/// a clear `ProviderNotConfigured` instead of a panic or a silently
/// missing button.
pub struct StubAuthProvider {
    platform: Platform,
    reason: String,
}

impl StubAuthProvider {
    pub fn new(platform: Platform, reason: impl Into<String>) -> Self {
        Self {
            platform,
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl PlatformAuthProvider for StubAuthProvider {
    fn platform(&self) -> Platform {
        self.platform
    }

    async fn authenticate(
        &self,
        _session: AuthSession,
        _cancel: oneshot::Receiver<()>,
    ) -> Result<ConnectedIdentity, AuthError> {
        Err(AuthError::ProviderNotConfigured {
            detail: self.reason.clone(),
        })
    }
}
