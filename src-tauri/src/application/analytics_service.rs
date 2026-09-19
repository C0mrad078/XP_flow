use crate::application::credential_acquisition_service::CredentialAcquisitionService;
use crate::domain::analytics::{AnalyticsCapabilities, PublicationMetricSnapshot};
use crate::domain::errors::DomainResult;
use crate::domain::platform::Platform;
use crate::domain::ports::analytics_provider::unsupported_capabilities;
use crate::domain::ports::analytics_provider::AnalyticsProvider;
use crate::domain::ports::repositories::AnalyticsRepository;
use crate::domain::ports::repositories::{PlatformAccountRepository, PublicationRepository};
use crate::infrastructure::connectors::youtube::analytics::YouTubeAnalyticsProvider;
use chrono::{Duration, Utc};
use std::sync::Arc;
use uuid::Uuid;

pub struct AnalyticsService {
    repo: Arc<dyn AnalyticsRepository>,
    publication_repo: Arc<dyn PublicationRepository>,
    account_repo: Arc<dyn PlatformAccountRepository>,
    credentials: Arc<CredentialAcquisitionService>,
}
impl AnalyticsService {
    pub fn new(
        repo: Arc<dyn AnalyticsRepository>,
        publication_repo: Arc<dyn PublicationRepository>,
        account_repo: Arc<dyn PlatformAccountRepository>,
        credentials: Arc<CredentialAcquisitionService>,
    ) -> Self {
        Self {
            repo,
            publication_repo,
            account_repo,
            credentials,
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
        let mut snapshot = if account.platform == Platform::YouTube {
            let token = self
                .credentials
                .acquire(&account)
                .await
                .map_err(|e| crate::domain::errors::DomainError::Validation(e.to_string()))?;
            let provider = YouTubeAnalyticsProvider::new();
            provider
                .fetch_publication_metrics(&token, &remote_id)
                .await
                .map_err(|e| crate::domain::errors::DomainError::Validation(e.to_string()))?
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
