use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::platform::Platform;
use crate::domain::ports::repositories::PublicationConsentRepository;
use crate::domain::publishing::{ApprovalSource, PublicationConsent};

use super::parse_dt;

pub struct SqlitePublicationConsentRepository {
    pool: SqlitePool,
}

impl SqlitePublicationConsentRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn map_repo_err(err: sqlx::Error) -> DomainError {
    DomainError::Repository(err.to_string())
}

const SELECT_COLUMNS: &str =
    "id, publication_id, provider, approved_at, approved_metadata_hash, approval_source, created_at";

fn row_to_consent(row: &sqlx::sqlite::SqliteRow) -> Result<PublicationConsent, DomainError> {
    Ok(PublicationConsent {
        id: Uuid::parse_str(&row.try_get::<String, _>("id").map_err(map_repo_err)?)
            .unwrap_or_default(),
        publication_id: Uuid::parse_str(
            &row.try_get::<String, _>("publication_id")
                .map_err(map_repo_err)?,
        )
        .unwrap_or_default(),
        provider: row
            .try_get::<String, _>("provider")
            .map_err(map_repo_err)?
            .parse::<Platform>()
            .map_err(DomainError::Validation)?,
        approved_at: parse_dt(
            &row.try_get::<String, _>("approved_at")
                .map_err(map_repo_err)?,
        ),
        approved_metadata_hash: row
            .try_get("approved_metadata_hash")
            .map_err(map_repo_err)?,
        approval_source: row
            .try_get::<String, _>("approval_source")
            .map_err(map_repo_err)?
            .parse::<ApprovalSource>()
            .map_err(DomainError::Validation)?,
        created_at: parse_dt(
            &row.try_get::<String, _>("created_at")
                .map_err(map_repo_err)?,
        ),
    })
}

#[async_trait]
impl PublicationConsentRepository for SqlitePublicationConsentRepository {
    async fn create(&self, consent: &PublicationConsent) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO publication_consent (id, publication_id, provider, approved_at, approved_metadata_hash, \
             approval_source, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(consent.id.to_string())
        .bind(consent.publication_id.to_string())
        .bind(consent.provider.as_str())
        .bind(consent.approved_at.to_rfc3339())
        .bind(&consent.approved_metadata_hash)
        .bind(consent.approval_source.as_str())
        .bind(consent.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn latest_for_publication(
        &self,
        publication_id: Uuid,
    ) -> DomainResult<Option<PublicationConsent>> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM publication_consent WHERE publication_id = ? ORDER BY created_at DESC LIMIT 1"
        ))
        .bind(publication_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_repo_err)?;
        row.as_ref().map(row_to_consent).transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::publishing::hash_publication_metadata;
    use crate::test_support::{
        seed_channel, seed_publication, seed_video, seed_workspace_and_source, temp_pool,
    };

    async fn seed_publication_id(pool: &SqlitePool) -> Uuid {
        let (workspace_id, source_id) = seed_workspace_and_source(pool).await;
        let channel_id = seed_channel(pool, workspace_id, "Channel").await;
        let video_id = seed_video(pool, workspace_id, source_id, Some(channel_id), "video").await;
        seed_publication(pool, workspace_id, channel_id, video_id).await
    }

    #[tokio::test]
    async fn latest_for_publication_returns_the_most_recent_approval() {
        let pool = temp_pool("consent-repo").await;
        let publication_id = seed_publication_id(&pool).await;
        let repo = SqlitePublicationConsentRepository::new(pool);

        let hash_a = hash_publication_metadata("Title", "Desc", &[], "{}");
        let consent_a = PublicationConsent::new(
            publication_id,
            Platform::TikTok,
            hash_a,
            ApprovalSource::AddToQueue,
        );
        repo.create(&consent_a).await.unwrap();

        let hash_b = hash_publication_metadata("New title", "Desc", &[], "{}");
        let consent_b = PublicationConsent::new(
            publication_id,
            Platform::TikTok,
            hash_b.clone(),
            ApprovalSource::ManualSchedule,
        );
        repo.create(&consent_b).await.unwrap();

        let latest = repo
            .latest_for_publication(publication_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(latest.approved_metadata_hash, hash_b);
        assert!(latest.covers(&hash_b));
    }

    #[tokio::test]
    async fn no_consent_yet_returns_none() {
        let pool = temp_pool("consent-repo-empty").await;
        let repo = SqlitePublicationConsentRepository::new(pool);
        assert!(repo
            .latest_for_publication(Uuid::new_v4())
            .await
            .unwrap()
            .is_none());
    }
}
