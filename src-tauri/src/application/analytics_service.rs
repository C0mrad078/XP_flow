use crate::application::credential_acquisition_service::CredentialAcquisitionService;
use crate::application::provider_rate_limit_service::ProviderRateLimitService;
use crate::domain::analytics::{
    AnalyticsCapabilities, AnalyticsSyncState, PublicationMetricSnapshot,
};
use crate::domain::errors::DomainResult;
use crate::domain::platform::Platform;
use crate::domain::ports::analytics_provider::unsupported_capabilities;
use crate::domain::ports::analytics_provider::AnalyticsProvider;
use crate::domain::ports::repositories::AnalyticsRepository;
use crate::domain::ports::repositories::{PlatformAccountRepository, PublicationRepository};
use crate::domain::publication::PublicationStatus;
use crate::domain::publication_query::{PublicationListQuery, QueueSort};
use crate::domain::publishing::RateLimitOperation;
use crate::infrastructure::connectors::youtube::analytics::YouTubeAnalyticsProvider;
use chrono::{Duration, Utc};
use std::sync::Arc;
use uuid::Uuid;

pub struct AnalyticsService {
    repo: Arc<dyn AnalyticsRepository>,
    publication_repo: Arc<dyn PublicationRepository>,
    account_repo: Arc<dyn PlatformAccountRepository>,
    credentials: Arc<CredentialAcquisitionService>,
    rate_limits: Arc<ProviderRateLimitService>,
}
impl AnalyticsService {
    pub fn new(
        repo: Arc<dyn AnalyticsRepository>,
        publication_repo: Arc<dyn PublicationRepository>,
        account_repo: Arc<dyn PlatformAccountRepository>,
        credentials: Arc<CredentialAcquisitionService>,
        rate_limits: Arc<ProviderRateLimitService>,
    ) -> Self {
        Self {
            repo,
            publication_repo,
            account_repo,
            credentials,
            rate_limits,
        }
    }
    pub async fn publication_snapshots(
        &self,
        id: Uuid,
        days: i64,
    ) -> DomainResult<Vec<PublicationMetricSnapshot>> {
        self.repo
            .list_publication_snapshots(
                id,
                Utc::now() - Duration::days(days.clamp(1, 90)),
                Utc::now() + Duration::seconds(1),
            )
            .await
    }
    pub fn capabilities(platform: Platform) -> AnalyticsCapabilities {
        if platform == Platform::YouTube {
            AnalyticsCapabilities {
                platform,
                publication_views: true,
                publication_likes: true,
                publication_comments: true,
                publication_shares: false,
                channel_followers: false,
            }
        } else {
            unsupported_capabilities(platform)
        }
    }

    pub async fn sync_publication(
        &self,
        publication_id: Uuid,
    ) -> DomainResult<PublicationMetricSnapshot> {
        let publication = self
            .publication_repo
            .get(publication_id)
            .await?
            .ok_or_else(|| crate::domain::errors::DomainError::NotFound {
                entity: "Publication",
                id: publication_id.to_string(),
            })?;
        let account_id = publication.platform_account_id.ok_or_else(|| {
            crate::domain::errors::DomainError::Validation(
                "publication has no platform account".into(),
            )
        })?;
        let account = self.account_repo.get(account_id).await?.ok_or_else(|| {
            crate::domain::errors::DomainError::NotFound {
                entity: "PlatformAccount",
                id: account_id.to_string(),
            }
        })?;
        let remote_id = publication.remote_id.clone().ok_or_else(|| {
            crate::domain::errors::DomainError::Validation("publication has no remote ID".into())
        })?;
        if !self
            .rate_limits
            .can_execute(account.id, RateLimitOperation::Analytics)
            .await
        {
            return Err(crate::domain::errors::DomainError::Validation(
                "analytics provider rate limit is active".into(),
            ));
        }
        let mut snapshot = if account.platform == Platform::YouTube {
            let token = self
                .credentials
                .acquire(&account)
                .await
                .map_err(|e| crate::domain::errors::DomainError::Validation(e.to_string()))?;
            let provider = YouTubeAnalyticsProvider::new();
            match provider.fetch_publication_metrics(&token, &remote_id).await {
                Ok(snapshot) => snapshot,
                Err(crate::domain::publishing::PublishError::RateLimited {
                    retry_after_seconds,
                }) => {
                    let _ = self
                        .rate_limits
                        .record_rate_limited(
                            account.id,
                            RateLimitOperation::Analytics,
                            retry_after_seconds,
                        )
                        .await;
                    return Err(crate::domain::errors::DomainError::Validation(
                        "analytics provider rate limited".into(),
                    ));
                }
                Err(error) => {
                    return Err(crate::domain::errors::DomainError::Validation(
                        error.to_string(),
                    ));
                }
            }
        } else {
            PublicationMetricSnapshot {
                id: Uuid::new_v4(),
                publication_id,
                provider: account.platform,
                captured_at: Utc::now(),
                views: None,
                likes: None,
                comments: None,
                shares: None,
                availability: crate::domain::analytics::AnalyticsAvailability::Unsupported,
                error_code: Some("PROVIDER_ANALYTICS_UNSUPPORTED".into()),
            }
        };
        snapshot.publication_id = publication_id;
        self.repo.insert_publication_snapshot(&snapshot).await?;
        Ok(snapshot)
    }

    /// Performs one bounded analytics sweep. The scheduler calls this from a
    /// single centralized task; it never creates one timer per publication.
    pub async fn sync_workspace(
        &self,
        workspace_id: Uuid,
        max_publications: usize,
    ) -> DomainResult<usize> {
        let accounts = self.account_repo.list_for_workspace(workspace_id).await?;
        let mut synced = 0;
        for account in accounts
            .into_iter()
            .filter(|account| account.platform == Platform::YouTube)
        {
            let previous_state = self.repo.get_sync_state(account.id).await?;
            if let Some(state) = &previous_state {
                if state.next_allowed_at.is_some_and(|at| at > Utc::now()) {
                    continue;
                }
            }
            if !self
                .rate_limits
                .can_execute(account.id, RateLimitOperation::Analytics)
                .await
            {
                continue;
            }
            let query = PublicationListQuery {
                workspace_id,
                search: None,
                channel_id: None,
                platform_account_id: Some(account.id),
                platform: Some(account.platform),
                priority: None,
                statuses: Some(vec![PublicationStatus::Published]),
                requires_attention: false,
                sort: QueueSort::NewestFirst,
                page: 0,
                page_size: max_publications.min(20) as i64,
            };
            let page = self.publication_repo.list_paginated(&query).await?;
            let item_count = page.items.len();
            let mut failures = 0;
            let mut last_error = None;
            for publication in page.items {
                match self.sync_publication(publication.id).await {
                    Ok(_) => synced += 1,
                    Err(error) => {
                        failures += 1;
                        last_error = Some(error.to_string());
                    }
                }
            }
            let now = Utc::now();
            self.repo
                .upsert_sync_state(&AnalyticsSyncState {
                    platform_account_id: account.id,
                    provider: account.platform,
                    last_attempted_at: Some(now),
                    last_successful_at: (failures == 0 && item_count > 0)
                        .then_some(now)
                        .or_else(|| previous_state.and_then(|state| state.last_successful_at)),
                    next_allowed_at: Some(now + Duration::minutes(15)),
                    last_error,
                })
                .await?;
        }
        Ok(synced)
    }
}

#[cfg(test)]
mod tests {
    use super::AnalyticsService;
    use crate::domain::platform::Platform;

    #[test]
    fn unsupported_providers_are_not_reported_as_zero_metrics() {
        let capabilities = AnalyticsService::capabilities(Platform::TikTok);
        assert!(!capabilities.publication_views);
        assert!(!capabilities.channel_followers);
    }

    #[test]
    fn youtube_exposes_only_statistics_supported_by_the_adapter() {
        let capabilities = AnalyticsService::capabilities(Platform::YouTube);
        assert!(capabilities.publication_views);
        assert!(capabilities.publication_likes);
        assert!(capabilities.publication_comments);
        assert!(!capabilities.publication_shares);
    }
}
