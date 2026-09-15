use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use super::platform_account::{PlatformAccount, PlatformAccountStatus};

/// How soon before `access_expires_at` an account is surfaced as "Token
/// Expiring" rather than "Healthy" (section 38's refresh buffer, reused
/// here for the *display* signal — the actual refresh job uses the same
/// buffer via `TokenLifecycleService`, see application/token_lifecycle_service.rs).
pub const TOKEN_EXPIRING_BUFFER: Duration = Duration::hours(24);

/// A derived, display-only signal (section 32) — never confused with
/// `PlatformAccountStatus` (persisted lifecycle) or `ChannelStatus`/queue
/// health (unrelated concepts that happen to also use the word "health").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionHealth {
    Healthy,
    TokenExpiring,
    PermissionMissing,
    RefreshRequired,
    Disconnected,
    ProviderError,
}

impl ConnectionHealth {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectionHealth::Healthy => "healthy",
            ConnectionHealth::TokenExpiring => "token_expiring",
            ConnectionHealth::PermissionMissing => "permission_missing",
            ConnectionHealth::RefreshRequired => "refresh_required",
            ConnectionHealth::Disconnected => "disconnected",
            ConnectionHealth::ProviderError => "provider_error",
        }
    }
}

/// Derives connection health from a `PlatformAccount`'s current stored
/// state — never persisted itself, always recomputed (same "derive, don't
/// store" discipline as Phase 3's `Publication::is_overdue`).
pub fn derive_connection_health(account: &PlatformAccount, now: DateTime<Utc>) -> ConnectionHealth {
    match account.status {
        PlatformAccountStatus::NotConfigured | PlatformAccountStatus::Revoked => {
            ConnectionHealth::Disconnected
        }
        PlatformAccountStatus::ReauthRequired => ConnectionHealth::RefreshRequired,
        PlatformAccountStatus::PermissionMissing => ConnectionHealth::PermissionMissing,
        PlatformAccountStatus::Error => ConnectionHealth::ProviderError,
        // Mid-flight states inherit the account's last known-good health
        // rather than reporting something scary while a normal background
        // refresh is simply in progress.
        PlatformAccountStatus::Connecting
        | PlatformAccountStatus::Refreshing
        | PlatformAccountStatus::Connected => match account.access_expires_at {
            Some(expires_at) if expires_at <= now + TOKEN_EXPIRING_BUFFER => {
                ConnectionHealth::TokenExpiring
            }
            _ => ConnectionHealth::Healthy,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::platform::Platform;
    use uuid::Uuid;

    fn sample_account(status: PlatformAccountStatus) -> PlatformAccount {
        let mut account = PlatformAccount::new(Uuid::new_v4(), Uuid::new_v4(), Platform::YouTube);
        account.status = status;
        account
    }

    #[test]
    fn not_configured_is_disconnected() {
        assert_eq!(
            derive_connection_health(
                &sample_account(PlatformAccountStatus::NotConfigured),
                Utc::now()
            ),
            ConnectionHealth::Disconnected
        );
    }

    #[test]
    fn connected_with_far_future_expiry_is_healthy() {
        let mut account = sample_account(PlatformAccountStatus::Connected);
        account.access_expires_at = Some(Utc::now() + Duration::days(30));
        assert_eq!(
            derive_connection_health(&account, Utc::now()),
            ConnectionHealth::Healthy
        );
    }

    #[test]
    fn connected_with_near_expiry_is_token_expiring() {
        let mut account = sample_account(PlatformAccountStatus::Connected);
        account.access_expires_at = Some(Utc::now() + Duration::hours(1));
        assert_eq!(
            derive_connection_health(&account, Utc::now()),
            ConnectionHealth::TokenExpiring
        );
    }

    #[test]
    fn connected_with_no_expiry_set_is_healthy() {
        let account = sample_account(PlatformAccountStatus::Connected);
        assert_eq!(
            derive_connection_health(&account, Utc::now()),
            ConnectionHealth::Healthy
        );
    }

    #[test]
    fn reauth_required_maps_to_refresh_required() {
        assert_eq!(
            derive_connection_health(
                &sample_account(PlatformAccountStatus::ReauthRequired),
                Utc::now()
            ),
            ConnectionHealth::RefreshRequired
        );
    }

    #[test]
    fn revoked_is_disconnected_not_error() {
        assert_eq!(
            derive_connection_health(&sample_account(PlatformAccountStatus::Revoked), Utc::now()),
            ConnectionHealth::Disconnected
        );
    }
}
