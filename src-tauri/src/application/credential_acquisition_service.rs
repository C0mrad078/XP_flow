use std::collections::HashMap;
use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::application::platform_auth_service::PlatformAuthService;
use crate::domain::platform::Platform;
use crate::domain::platform_account::PlatformAccount;
use crate::domain::ports::platform_connector::PlatformConnector;
use crate::domain::publishing::PublishError;

/// A short buffer for a token about to be *used right now* — deliberately
/// tighter than `connection_health`'s 24-hour "should show a warning
/// badge" buffer, since this check happens immediately before an upload
/// starts, not as a standing UI signal.
const ACQUIRE_REFRESH_BUFFER: Duration = Duration::minutes(10);

/// The single authoritative point every publishing code path goes
/// through for a usable provider access token (section 155) — no
/// provider uploader, chunk planner, or job ever calls
/// `PlatformAuthService`/`BrokerClient`/`SecureStorage` directly. This is
/// what keeps the Phase 5 concurrency hardening (section 154) meaningful:
/// if two upload attempts could each independently decide to refresh,
/// the CAS guard on `PlatformAuthService::refresh` would still prevent
/// corruption, but this service is what makes that the *only* place a
/// refresh can ever be triggered from.
pub struct CredentialAcquisitionService {
    connectors: HashMap<Platform, Arc<dyn PlatformConnector>>,
    platform_auth_service: Arc<PlatformAuthService>,
}

impl CredentialAcquisitionService {
    pub fn new(
        connectors: HashMap<Platform, Arc<dyn PlatformConnector>>,
        platform_auth_service: Arc<PlatformAuthService>,
    ) -> Self {
        Self {
            connectors,
            platform_auth_service,
        }
    }

    /// Returns a currently-usable access token for `account`. For
    /// YouTube, refreshes first (through `PlatformAuthService`'s CAS-
    /// guarded path) if the stored token is expiring within the buffer;
    /// for TikTok/Kwai, the broker's own access-token endpoint already
    /// refreshes server-side when needed, so no separate desktop-side
    /// decision is made for those two.
    pub async fn acquire(&self, account: &PlatformAccount) -> Result<String, PublishError> {
        let connector =
            self.connectors
                .get(&account.platform)
                .ok_or_else(|| PublishError::Internal {
                    detail: format!("no connector registered for {}", account.platform),
                })?;

        if account.platform == Platform::YouTube {
            let needs_refresh = account
                .access_expires_at
                .is_none_or(|expires_at| expires_at <= Utc::now() + ACQUIRE_REFRESH_BUFFER);
            if needs_refresh {
                self.platform_auth_service
                    .refresh(account.id)
                    .await
                    .map_err(map_auth_err)?;
            }
        }

        connector
            .acquire_access_token(account)
            .await
            .map_err(map_auth_err)
    }
}

fn map_auth_err(err: crate::domain::auth_error::AuthError) -> PublishError {
    use crate::domain::auth_error::AuthError;
    match err {
        AuthError::TokenRevoked => PublishError::AuthRevoked,
        AuthError::PermissionMissing { capability } => {
            PublishError::PermissionMissing { capability }
        }
        AuthError::ProviderRateLimited {
            retry_after_seconds,
        } => PublishError::RateLimited {
            retry_after_seconds,
        },
        AuthError::NetworkOffline => PublishError::NetworkTransient,
        AuthError::ProviderUnavailable { .. } | AuthError::BrokerUnavailable => {
            PublishError::ProviderServerError { status: None }
        }
        _ => PublishError::AuthExpired,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::auth_error::AuthError;

    #[test]
    fn revoked_maps_to_auth_revoked_not_a_generic_internal_error() {
        assert!(matches!(
            map_auth_err(AuthError::TokenRevoked),
            PublishError::AuthRevoked
        ));
    }

    #[test]
    fn rate_limited_preserves_the_retry_after_hint() {
        let mapped = map_auth_err(AuthError::ProviderRateLimited {
            retry_after_seconds: Some(120),
        });
        assert!(matches!(
            mapped,
            PublishError::RateLimited {
                retry_after_seconds: Some(120)
            }
        ));
    }

    #[test]
    fn network_offline_is_retryable_transient() {
        assert!(map_auth_err(AuthError::NetworkOffline).is_retryable());
    }
}
