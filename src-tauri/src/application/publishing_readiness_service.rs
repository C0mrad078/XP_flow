use std::collections::HashMap;
use std::sync::Arc;

use uuid::Uuid;

use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::platform::Platform;
use crate::domain::ports::platform_publisher::PlatformPublisher;
use crate::domain::ports::repositories::{
    PlatformAccountRepository, PublicationConsentRepository, PublicationRepository, VideoRepository,
};
use crate::domain::publication::PublicationStatus;
use crate::domain::publishing::{requires_express_consent, RenderedMetadata};
use crate::domain::readiness::{compute_readiness, ReadinessInputs, ReadinessIssue};

/// Assembles [`ReadinessInputs`] from real, already-persisted state and
/// runs the pure `compute_readiness` (section 74/75) — this is the "why
/// isn't this publication going out" answer the Queue/Today UI and the
/// Publication Details drawer surface directly, computed fresh on every
/// call rather than cached, so it can never drift from what the engine
/// itself would actually do.
pub struct PublishingReadinessService {
    publication_repo: Arc<dyn PublicationRepository>,
    video_repo: Arc<dyn VideoRepository>,
    platform_account_repo: Arc<dyn PlatformAccountRepository>,
    consent_repo: Arc<dyn PublicationConsentRepository>,
    publishers: HashMap<Platform, Arc<dyn PlatformPublisher>>,
}

impl PublishingReadinessService {
    pub fn new(
        publication_repo: Arc<dyn PublicationRepository>,
        video_repo: Arc<dyn VideoRepository>,
        platform_account_repo: Arc<dyn PlatformAccountRepository>,
        consent_repo: Arc<dyn PublicationConsentRepository>,
        publishers: HashMap<Platform, Arc<dyn PlatformPublisher>>,
    ) -> Self {
        Self {
            publication_repo,
            video_repo,
            platform_account_repo,
            consent_repo,
            publishers,
        }
    }

    pub async fn compute(&self, publication_id: Uuid) -> DomainResult<Vec<ReadinessIssue>> {
        let Some(publication) = self.publication_repo.get(publication_id).await? else {
            return Err(DomainError::NotFound {
                entity: "publication",
                id: publication_id.to_string(),
            });
        };
        let video = self.video_repo.get(publication.video_id).await?;
        let account = match publication.platform_account_id {
            Some(id) => self.platform_account_repo.get(id).await?,
            None => None,
        };

        let metadata = RenderedMetadata {
            title: publication.title.clone(),
            description: publication.description.clone().unwrap_or_default(),
            hashtags: publication.hashtags.clone(),
            provider_options: serde_json::json!({}),
        };
        let metadata_valid = self
            .publishers
            .get(&publication.platform)
            .map(|publisher| publisher.validate_metadata(&metadata).is_ok());

        let consent_ok = if requires_express_consent(publication.platform) {
            let current_hash = metadata.consent_hash();
            let covers = self
                .consent_repo
                .latest_for_publication(publication_id)
                .await?
                .is_some_and(|consent| consent.covers(&current_hash));
            Some(covers)
        } else {
            None
        };

        let inputs = ReadinessInputs {
            platform_account: account.as_ref(),
            video_availability: video.as_ref().map(|v| v.availability_status),
            video_validation: video.as_ref().map(|v| v.validation_status),
            locked: publication.locked,
            already_published: publication.status == PublicationStatus::Published,
            metadata_valid,
            consent_ok,
            // Section 105-109's provider-app-review tracking and
            // section 78/129's rate-limit-window persistence aren't
            // wired to any writer yet (both pending Phase 5 work) —
            // `None` here means "not checked," never a false "cleared."
            platform_approved: None,
            rate_limited_until: None,
        };
        Ok(compute_readiness(&inputs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::capability::Capability;
    use crate::domain::platform_account::{PlatformAccount, PlatformAccountStatus};
    use crate::domain::publication::Publication;
    use crate::domain::video::Video;
    use crate::domain::video_status::VideoPriority;
    use crate::infrastructure::publishing::{FakePublisher, FakeScenario};
    use crate::infrastructure::repositories::{
        SqlitePlatformAccountRepository, SqlitePublicationConsentRepository,
        SqlitePublicationRepository, SqliteVideoRepository,
    };
    use crate::test_support::*;

    async fn build_service(pool: &sqlx::SqlitePool) -> PublishingReadinessService {
        let mut publishers: HashMap<Platform, Arc<dyn PlatformPublisher>> = HashMap::new();
        publishers.insert(
            Platform::YouTube,
            Arc::new(FakePublisher::new(Platform::YouTube, FakeScenario::Success)),
        );
        PublishingReadinessService::new(
            Arc::new(SqlitePublicationRepository::new(pool.clone())),
            Arc::new(SqliteVideoRepository::new(pool.clone())),
            Arc::new(SqlitePlatformAccountRepository::new(pool.clone())),
            Arc::new(SqlitePublicationConsentRepository::new(pool.clone())),
            publishers,
        )
    }

    async fn seed_publication_with_video(
        pool: &sqlx::SqlitePool,
        workspace_id: Uuid,
        channel_id: Uuid,
        source_id: Uuid,
    ) -> Uuid {
        let dir = temp_dir("readiness-service-video");
        let path = write_fake_video(&dir, "clip.mp4", b"fake video bytes");
        let mut video = Video::new(
            workspace_id,
            source_id,
            Some(channel_id),
            "clip.mp4",
            "Clip",
            path.display().to_string(),
            path.metadata().unwrap().len() as i64,
            "mp4",
        );
        // A freshly ingested video starts `ValidationStatus::Pending`
        // (not yet probed) — these fixtures care about readiness given a
        // video that has already cleared validation, not about the probe
        // pipeline itself.
        video.validation_status = crate::domain::video_status::ValidationStatus::Valid;
        use crate::domain::ports::repositories::VideoRepository;
        SqliteVideoRepository::new(pool.clone())
            .create(&video)
            .await
            .unwrap();

        let publication = Publication::new(
            workspace_id,
            video.id,
            channel_id,
            Platform::YouTube,
            "Great clip",
            VideoPriority::Normal,
        );
        use crate::domain::ports::repositories::PublicationRepository;
        SqlitePublicationRepository::new(pool.clone())
            .create(&publication)
            .await
            .unwrap();
        publication.id
    }

    #[tokio::test]
    async fn a_publication_with_no_account_reports_account_not_connected() {
        let pool = temp_pool("readiness-no-account").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Channel").await;
        let publication_id =
            seed_publication_with_video(&pool, workspace_id, channel_id, source_id).await;

        let service = build_service(&pool).await;
        let issues = service.compute(publication_id).await.unwrap();
        assert!(issues.contains(&crate::domain::readiness::ReadinessIssue::AccountNotConnected));
    }

    #[tokio::test]
    async fn a_fully_ready_publication_has_no_issues() {
        let pool = temp_pool("readiness-fully-ready").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Channel").await;
        let publication_id =
            seed_publication_with_video(&pool, workspace_id, channel_id, source_id).await;

        let mut account = PlatformAccount::new(workspace_id, channel_id, Platform::YouTube);
        account.status = PlatformAccountStatus::Connected;
        account.capabilities = vec![Capability::ReadProfile, Capability::UploadVideo];
        use crate::domain::ports::repositories::PlatformAccountRepository;
        SqlitePlatformAccountRepository::new(pool.clone())
            .create(&account)
            .await
            .unwrap();

        use crate::domain::ports::repositories::PublicationRepository;
        let publication_repo = SqlitePublicationRepository::new(pool.clone());
        let mut publication = publication_repo.get(publication_id).await.unwrap().unwrap();
        publication.platform_account_id = Some(account.id);
        publication_repo.update(&publication).await.unwrap();

        let service = build_service(&pool).await;
        let issues = service.compute(publication_id).await.unwrap();
        assert!(issues.is_empty(), "{issues:?}");
    }

    #[tokio::test]
    async fn a_locked_publication_is_flagged_regardless_of_everything_else() {
        let pool = temp_pool("readiness-locked").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Channel").await;
        let publication_id =
            seed_publication_with_video(&pool, workspace_id, channel_id, source_id).await;

        use crate::domain::ports::repositories::PublicationRepository;
        let publication_repo = SqlitePublicationRepository::new(pool.clone());
        let mut publication = publication_repo.get(publication_id).await.unwrap().unwrap();
        publication.locked = true;
        publication_repo.update(&publication).await.unwrap();

        let service = build_service(&pool).await;
        let issues = service.compute(publication_id).await.unwrap();
        assert!(issues.contains(&crate::domain::readiness::ReadinessIssue::PublicationLocked));
    }
}
