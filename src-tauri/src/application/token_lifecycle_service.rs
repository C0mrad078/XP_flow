use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::application::platform_auth_service::PlatformAuthService;
use crate::domain::connection_health::TOKEN_EXPIRING_BUFFER;
use crate::domain::ports::repositories::PlatformAccountRepository;

#[derive(Debug, Default, Clone, Copy)]
pub struct RefreshSweepSummary {
    pub refreshed: u32,
    pub failed: u32,
    pub skipped: u32,
}

/// Detects and refreshes soon-to-expire credentials (section 37-40)
/// before they actually expire, rather than waiting for an API call to
/// fail. Deliberately thin: all the actual provider-specific work lives in
/// `PlatformAuthService::refresh` / the `PlatformConnector`
/// implementations — this service only decides *which* accounts are due
/// and sweeps them, on a schedule driven by `JobRunner`
/// (`spawn_periodic_token_refresh`) rather than a second background-task
/// system (section 39).
pub struct TokenLifecycleService {
    platform_account_repo: Arc<dyn PlatformAccountRepository>,
    platform_auth_service: Arc<PlatformAuthService>,
}

impl TokenLifecycleService {
    pub fn new(
        platform_account_repo: Arc<dyn PlatformAccountRepository>,
        platform_auth_service: Arc<PlatformAuthService>,
    ) -> Self {
        Self {
            platform_account_repo,
            platform_auth_service,
        }
    }

    /// Refreshes every `Connected` account whose access token expires
    /// within `TOKEN_EXPIRING_BUFFER` — the same buffer the display-only
    /// `ConnectionHealth::TokenExpiring` signal uses, so "the UI shows
    /// this as expiring soon" and "this job will refresh it soon" always
    /// agree.
    pub async fn refresh_expiring_accounts(&self) -> RefreshSweepSummary {
        let mut summary = RefreshSweepSummary::default();

        let due = match self
            .platform_account_repo
            .list_due_for_refresh(Utc::now() + TOKEN_EXPIRING_BUFFER)
            .await
        {
            Ok(accounts) => accounts,
            Err(err) => {
                tracing::error!(%err, "failed to list platform accounts due for refresh");
                return summary;
            }
        };

        for account in due {
            match self.platform_auth_service.refresh(account.id).await {
                Ok(_) => summary.refreshed += 1,
                Err(crate::domain::auth_error::AuthError::TokenExchangeFailed { detail })
                    if detail.contains("already in progress") =>
                {
                    summary.skipped += 1;
                }
                Err(err) => {
                    tracing::warn!(account_id = %account.id, platform = %account.platform, error = %err, "scheduled token refresh failed");
                    summary.failed += 1;
                }
            }
        }

        summary
    }

    /// Exposed for a manual "check connection health now" trigger,
    /// bounded to a single account rather than the whole sweep.
    pub async fn refresh_if_due(&self, account_id: uuid::Uuid) -> Option<Duration> {
        let account = self.platform_account_repo.get(account_id).await.ok()??;
        let due_at = account.access_expires_at?;
        if due_at <= Utc::now() + TOKEN_EXPIRING_BUFFER {
            let _ = self.platform_auth_service.refresh(account_id).await;
            None
        } else {
            Some(due_at - Utc::now())
        }
    }
}
