use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::domain::errors::DomainResult;
use crate::domain::ports::repositories::ProviderRateStateRepository;
use crate::domain::publishing::RateLimitOperation;

/// Used only when a provider signaled a rate limit without a real
/// `Retry-After` value (Phase 5.1 section 20: the provider's own timing
/// takes precedence *when it gives one* — this is the honest fallback
/// for when it didn't, not a guess dressed up as provider data).
const DEFAULT_RATE_LIMIT_WAIT_SECS: i64 = 60;

/// One provider account's currently-known rate-limit window for one
/// operation class — `None` means "not currently limited," not "never
/// limited" (the repository only remembers the most recent window).
#[derive(Debug, Clone, Serialize)]
pub struct ProviderRateLimitStatus {
    pub operation: RateLimitOperation,
    pub limited_until: Option<DateTime<Utc>>,
}

/// The one authoritative place provider rate-limit state is read and
/// written (Phase 5.1 section 19) — no uploader or connector calculates
/// or persists this itself; they only report `PublishError::RateLimited
/// { retry_after_seconds }` (or, for AUTH, an analogous signal) and the
/// engine records it here.
pub struct ProviderRateLimitService {
    repo: Arc<dyn ProviderRateStateRepository>,
}

impl ProviderRateLimitService {
    pub fn new(repo: Arc<dyn ProviderRateStateRepository>) -> Self {
        Self { repo }
    }

    /// Whether a request of this class is currently safe to send —
    /// `false` means a known window is still open and the caller must
    /// not make the request (section 22: never repeatedly hit a known
    /// limit).
    pub async fn can_execute(
        &self,
        platform_account_id: Uuid,
        operation: RateLimitOperation,
    ) -> bool {
        match self
            .repo
            .get_retry_after(platform_account_id, operation.as_str())
            .await
        {
            Ok(Some(retry_after)) => retry_after <= Utc::now(),
            _ => true,
        }
    }

    /// `None` if not currently limited; otherwise the real instant a
    /// request may be sent again.
    pub async fn get_next_allowed_at(
        &self,
        platform_account_id: Uuid,
        operation: RateLimitOperation,
    ) -> Option<DateTime<Utc>> {
        match self
            .repo
            .get_retry_after(platform_account_id, operation.as_str())
            .await
        {
            Ok(Some(retry_after)) if retry_after > Utc::now() => Some(retry_after),
            _ => None,
        }
    }

    /// Records a rate limit signaled by the provider, converting a
    /// relative `retry_after_seconds` (when it supplied one) into the
    /// absolute instant everything else here reasons about. Falls back
    /// to `DEFAULT_RATE_LIMIT_WAIT_SECS` only when the provider gave no
    /// timing at all.
    pub async fn record_rate_limited(
        &self,
        platform_account_id: Uuid,
        operation: RateLimitOperation,
        retry_after_seconds: Option<u64>,
    ) -> DomainResult<()> {
        let wait_seconds = retry_after_seconds
            .map(|s| s as i64)
            .unwrap_or(DEFAULT_RATE_LIMIT_WAIT_SECS);
        let retry_after = Utc::now() + Duration::seconds(wait_seconds);
        self.repo
            .record_rate_limit(platform_account_id, operation.as_str(), retry_after)
            .await
    }

    /// Clears a stale rate-limit window after a real success (section
    /// 19/85: "provider success clearing/updating state"). The
    /// underlying repository has no separate delete — recording a
    /// `retry_after` already in the past is indistinguishable from
    /// "cleared" to every reader (`can_execute`/`get_next_allowed_at`
    /// both compare against `now`), and stays within the repository's
    /// existing, honest contract (a `Retry-After`-derived timestamp,
    /// nothing more) rather than adding a second write path.
    pub async fn record_success(
        &self,
        platform_account_id: Uuid,
        operation: RateLimitOperation,
    ) -> DomainResult<()> {
        self.repo
            .record_rate_limit(
                platform_account_id,
                operation.as_str(),
                Utc::now() - Duration::seconds(1),
            )
            .await
    }

    /// The currently-known state across every wired operation class, for
    /// the `get_provider_rate_state` command (section 23/59).
    pub async fn get_state(&self, platform_account_id: Uuid) -> Vec<ProviderRateLimitStatus> {
        let mut out = Vec::new();
        for operation in [
            RateLimitOperation::Publish,
            RateLimitOperation::Status,
            RateLimitOperation::Auth,
        ] {
            let limited_until = self
                .get_next_allowed_at(platform_account_id, operation)
                .await;
            out.push(ProviderRateLimitStatus {
                operation,
                limited_until,
            });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repositories::SqliteProviderRateStateRepository;
    use crate::test_support::{
        seed_channel, seed_platform_account, seed_workspace_and_source, temp_pool,
    };

    #[tokio::test]
    async fn a_fresh_account_can_always_execute() {
        let pool = temp_pool("rate-limit-service-fresh").await;
        let (workspace_id, _source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Channel").await;
        let account_id = seed_platform_account(&pool, workspace_id, channel_id).await;
        let service =
            ProviderRateLimitService::new(Arc::new(SqliteProviderRateStateRepository::new(pool)));

        assert!(
            service
                .can_execute(account_id, RateLimitOperation::Publish)
                .await
        );
        assert!(service
            .get_next_allowed_at(account_id, RateLimitOperation::Publish)
            .await
            .is_none());
    }

    #[tokio::test]
    async fn recording_a_rate_limit_blocks_execution_until_it_expires() {
        let pool = temp_pool("rate-limit-service-blocked").await;
        let (workspace_id, _source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Channel").await;
        let account_id = seed_platform_account(&pool, workspace_id, channel_id).await;
        let service =
            ProviderRateLimitService::new(Arc::new(SqliteProviderRateStateRepository::new(pool)));

        service
            .record_rate_limited(account_id, RateLimitOperation::Publish, Some(120))
            .await
            .unwrap();

        assert!(
            !service
                .can_execute(account_id, RateLimitOperation::Publish)
                .await
        );
        let next = service
            .get_next_allowed_at(account_id, RateLimitOperation::Publish)
            .await
            .unwrap();
        assert!(next > Utc::now() + Duration::seconds(100));
        assert!(next <= Utc::now() + Duration::seconds(121));

        // A different operation class on the same account is unaffected.
        assert!(
            service
                .can_execute(account_id, RateLimitOperation::Status)
                .await
        );
    }

    #[tokio::test]
    async fn a_missing_provider_retry_after_falls_back_to_the_default_wait() {
        let pool = temp_pool("rate-limit-service-default-wait").await;
        let (workspace_id, _source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Channel").await;
        let account_id = seed_platform_account(&pool, workspace_id, channel_id).await;
        let service =
            ProviderRateLimitService::new(Arc::new(SqliteProviderRateStateRepository::new(pool)));

        service
            .record_rate_limited(account_id, RateLimitOperation::Publish, None)
            .await
            .unwrap();

        let next = service
            .get_next_allowed_at(account_id, RateLimitOperation::Publish)
            .await
            .unwrap();
        assert!(next > Utc::now());
        assert!(next <= Utc::now() + Duration::seconds(DEFAULT_RATE_LIMIT_WAIT_SECS + 1));
    }

    #[tokio::test]
    async fn recording_a_success_clears_a_previously_rate_limited_window() {
        let pool = temp_pool("rate-limit-service-cleared").await;
        let (workspace_id, _source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Channel").await;
        let account_id = seed_platform_account(&pool, workspace_id, channel_id).await;
        let service =
            ProviderRateLimitService::new(Arc::new(SqliteProviderRateStateRepository::new(pool)));

        service
            .record_rate_limited(account_id, RateLimitOperation::Publish, Some(600))
            .await
            .unwrap();
        assert!(
            !service
                .can_execute(account_id, RateLimitOperation::Publish)
                .await
        );

        service
            .record_success(account_id, RateLimitOperation::Publish)
            .await
            .unwrap();
        assert!(
            service
                .can_execute(account_id, RateLimitOperation::Publish)
                .await
        );
    }

    #[tokio::test]
    async fn get_state_reports_every_wired_operation_class() {
        let pool = temp_pool("rate-limit-service-state").await;
        let (workspace_id, _source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Channel").await;
        let account_id = seed_platform_account(&pool, workspace_id, channel_id).await;
        let service =
            ProviderRateLimitService::new(Arc::new(SqliteProviderRateStateRepository::new(pool)));

        service
            .record_rate_limited(account_id, RateLimitOperation::Publish, Some(60))
            .await
            .unwrap();

        let state = service.get_state(account_id).await;
        assert_eq!(state.len(), 3);
        let publish = state
            .iter()
            .find(|s| s.operation == RateLimitOperation::Publish)
            .unwrap();
        assert!(publish.limited_until.is_some());
        let status = state
            .iter()
            .find(|s| s.operation == RateLimitOperation::Status)
            .unwrap();
        assert!(status.limited_until.is_none());
    }
}
