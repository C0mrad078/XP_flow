use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::platform::Platform;
use crate::domain::ports::repositories::PublicationRepository;
use crate::domain::publication::{Publication, PublicationStatus};

use super::parse_dt;

pub struct SqlitePublicationRepository {
    pool: SqlitePool,
}

impl SqlitePublicationRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn map_repo_err(err: sqlx::Error) -> DomainError {
    DomainError::Repository(err.to_string())
}

fn row_to_publication(row: &sqlx::sqlite::SqliteRow) -> Result<Publication, DomainError> {
    let hashtags_json: String = row.try_get("hashtags_json").map_err(map_repo_err)?;
    let platform_account_id: Option<String> =
        row.try_get("platform_account_id").map_err(map_repo_err)?;
    let scheduled_at: Option<String> = row.try_get("scheduled_at").map_err(map_repo_err)?;
    let published_at: Option<String> = row.try_get("published_at").map_err(map_repo_err)?;

    Ok(Publication {
        id: Uuid::parse_str(&row.try_get::<String, _>("id").map_err(map_repo_err)?)
            .unwrap_or_default(),
        video_id: Uuid::parse_str(&row.try_get::<String, _>("video_id").map_err(map_repo_err)?)
            .unwrap_or_default(),
        channel_id: Uuid::parse_str(
            &row.try_get::<String, _>("channel_id")
                .map_err(map_repo_err)?,
        )
        .unwrap_or_default(),
        platform_account_id: platform_account_id.and_then(|s| Uuid::parse_str(&s).ok()),
        platform: row
            .try_get::<String, _>("platform")
            .map_err(map_repo_err)?
            .parse::<Platform>()
            .map_err(DomainError::Validation)?,
        status: row
            .try_get::<String, _>("status")
            .map_err(map_repo_err)?
            .parse::<PublicationStatus>()
            .map_err(DomainError::Validation)?,
        title: row.try_get("title").map_err(map_repo_err)?,
        description: row.try_get("description").map_err(map_repo_err)?,
        hashtags: serde_json::from_str(&hashtags_json).unwrap_or_default(),
        scheduled_at: scheduled_at.map(|s| parse_dt(&s)),
        published_at: published_at.map(|s| parse_dt(&s)),
        remote_id: row.try_get("remote_id").map_err(map_repo_err)?,
        retry_count: row.try_get("retry_count").map_err(map_repo_err)?,
        last_error: row.try_get("last_error").map_err(map_repo_err)?,
        created_at: parse_dt(
            &row.try_get::<String, _>("created_at")
                .map_err(map_repo_err)?,
        ),
        updated_at: parse_dt(
            &row.try_get::<String, _>("updated_at")
                .map_err(map_repo_err)?,
        ),
    })
}

const SELECT_COLUMNS: &str = "id, video_id, channel_id, platform_account_id, platform, status, title, \
     description, hashtags_json, scheduled_at, published_at, remote_id, retry_count, last_error, created_at, updated_at";

#[async_trait]
impl PublicationRepository for SqlitePublicationRepository {
    async fn create(&self, publication: &Publication) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO publications (id, video_id, channel_id, platform_account_id, platform, status, title, \
             description, hashtags_json, scheduled_at, published_at, remote_id, retry_count, last_error, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(publication.id.to_string())
        .bind(publication.video_id.to_string())
        .bind(publication.channel_id.to_string())
        .bind(publication.platform_account_id.map(|id| id.to_string()))
        .bind(publication.platform.as_str())
        .bind(publication.status.as_str())
        .bind(&publication.title)
        .bind(&publication.description)
        .bind(serde_json::to_string(&publication.hashtags).unwrap_or_else(|_| "[]".to_string()))
        .bind(publication.scheduled_at.map(|dt| dt.to_rfc3339()))
        .bind(publication.published_at.map(|dt| dt.to_rfc3339()))
        .bind(&publication.remote_id)
        .bind(publication.retry_count)
        .bind(&publication.last_error)
        .bind(publication.created_at.to_rfc3339())
        .bind(publication.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn update(&self, publication: &Publication) -> DomainResult<()> {
        sqlx::query(
            "UPDATE publications SET status = ?, title = ?, description = ?, hashtags_json = ?, \
             scheduled_at = ?, published_at = ?, remote_id = ?, retry_count = ?, last_error = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(publication.status.as_str())
        .bind(&publication.title)
        .bind(&publication.description)
        .bind(serde_json::to_string(&publication.hashtags).unwrap_or_else(|_| "[]".to_string()))
        .bind(publication.scheduled_at.map(|dt| dt.to_rfc3339()))
        .bind(publication.published_at.map(|dt| dt.to_rfc3339()))
        .bind(&publication.remote_id)
        .bind(publication.retry_count)
        .bind(&publication.last_error)
        .bind(publication.updated_at.to_rfc3339())
        .bind(publication.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn list_for_video(&self, video_id: Uuid) -> DomainResult<Vec<Publication>> {
        let rows = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM publications WHERE video_id = ? ORDER BY created_at ASC"
        ))
        .bind(video_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(map_repo_err)?;

        rows.iter().map(row_to_publication).collect()
    }
}
