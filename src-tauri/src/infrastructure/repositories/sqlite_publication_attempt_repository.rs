use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::platform::Platform;
use crate::domain::ports::repositories::PublicationAttemptRepository;
use crate::domain::publishing::{AttemptStatus, PublicationAttempt};

use super::parse_dt;

pub struct SqlitePublicationAttemptRepository {
    pool: SqlitePool,
}

impl SqlitePublicationAttemptRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn map_repo_err(err: sqlx::Error) -> DomainError {
    DomainError::Repository(err.to_string())
}

const SELECT_COLUMNS: &str = "id, publication_id, attempt_number, provider, status, started_at, completed_at, \
     bytes_total, bytes_uploaded, error_code, error_message, retryable, remote_operation_id, created_at";

fn row_to_attempt(row: &sqlx::sqlite::SqliteRow) -> Result<PublicationAttempt, DomainError> {
    let started_at: Option<String> = row.try_get("started_at").map_err(map_repo_err)?;
    let completed_at: Option<String> = row.try_get("completed_at").map_err(map_repo_err)?;
    let retryable: Option<i64> = row.try_get("retryable").map_err(map_repo_err)?;

    Ok(PublicationAttempt {
        id: Uuid::parse_str(&row.try_get::<String, _>("id").map_err(map_repo_err)?)
            .unwrap_or_default(),
        publication_id: Uuid::parse_str(
            &row.try_get::<String, _>("publication_id")
                .map_err(map_repo_err)?,
        )
        .unwrap_or_default(),
        attempt_number: row.try_get("attempt_number").map_err(map_repo_err)?,
        provider: row
            .try_get::<String, _>("provider")
            .map_err(map_repo_err)?
            .parse::<Platform>()
            .map_err(DomainError::Validation)?,
        status: row
            .try_get::<String, _>("status")
            .map_err(map_repo_err)?
            .parse::<AttemptStatus>()
            .map_err(DomainError::Validation)?,
        started_at: started_at.map(|s| parse_dt(&s)),
        completed_at: completed_at.map(|s| parse_dt(&s)),
        bytes_total: row.try_get("bytes_total").map_err(map_repo_err)?,
        bytes_uploaded: row.try_get("bytes_uploaded").map_err(map_repo_err)?,
        error_code: row.try_get("error_code").map_err(map_repo_err)?,
        error_message: row.try_get("error_message").map_err(map_repo_err)?,
        retryable: retryable.map(|v| v != 0),
        remote_operation_id: row.try_get("remote_operation_id").map_err(map_repo_err)?,
        created_at: parse_dt(
            &row.try_get::<String, _>("created_at")
                .map_err(map_repo_err)?,
        ),
    })
}

#[async_trait]
impl PublicationAttemptRepository for SqlitePublicationAttemptRepository {
    async fn create(&self, attempt: &PublicationAttempt) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO publication_attempts (id, publication_id, attempt_number, provider, status, started_at, \
             completed_at, bytes_total, bytes_uploaded, error_code, error_message, retryable, remote_operation_id, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(attempt.id.to_string())
        .bind(attempt.publication_id.to_string())
        .bind(attempt.attempt_number)
        .bind(attempt.provider.as_str())
        .bind(attempt.status.as_str())
        .bind(attempt.started_at.map(|dt| dt.to_rfc3339()))
        .bind(attempt.completed_at.map(|dt| dt.to_rfc3339()))
        .bind(attempt.bytes_total)
        .bind(attempt.bytes_uploaded)
        .bind(&attempt.error_code)
        .bind(&attempt.error_message)
        .bind(attempt.retryable)
        .bind(&attempt.remote_operation_id)
        .bind(attempt.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn update(&self, attempt: &PublicationAttempt) -> DomainResult<()> {
        sqlx::query(
            "UPDATE publication_attempts SET status = ?, started_at = ?, completed_at = ?, bytes_total = ?, \
             bytes_uploaded = ?, error_code = ?, error_message = ?, retryable = ?, remote_operation_id = ? \
             WHERE id = ?",
        )
        .bind(attempt.status.as_str())
        .bind(attempt.started_at.map(|dt| dt.to_rfc3339()))
        .bind(attempt.completed_at.map(|dt| dt.to_rfc3339()))
        .bind(attempt.bytes_total)
        .bind(attempt.bytes_uploaded)
        .bind(&attempt.error_code)
        .bind(&attempt.error_message)
        .bind(attempt.retryable)
        .bind(&attempt.remote_operation_id)
        .bind(attempt.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn get(&self, id: Uuid) -> DomainResult<Option<PublicationAttempt>> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM publication_attempts WHERE id = ?"
        ))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_repo_err)?;
        row.as_ref().map(row_to_attempt).transpose()
    }

    async fn list_for_publication(
        &self,
        publication_id: Uuid,
    ) -> DomainResult<Vec<PublicationAttempt>> {
        let rows = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM publication_attempts WHERE publication_id = ? ORDER BY attempt_number DESC"
        ))
        .bind(publication_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(map_repo_err)?;
        rows.iter().map(row_to_attempt).collect()
    }

    async fn max_attempt_number(&self, publication_id: Uuid) -> DomainResult<i32> {
        let row = sqlx::query(
            "SELECT COALESCE(MAX(attempt_number), 0) AS max_attempt FROM publication_attempts WHERE publication_id = ?",
        )
        .bind(publication_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(map_repo_err)?;
        row.try_get("max_attempt").map_err(map_repo_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    async fn create_then_list_round_trips_and_orders_newest_first() {
        let pool = temp_pool("attempt-repo").await;
        let publication_id = seed_publication_id(&pool).await;
        let repo = SqlitePublicationAttemptRepository::new(pool);

        let mut first = PublicationAttempt::new(publication_id, 1, Platform::YouTube);
        first.start();
        repo.create(&first).await.unwrap();

        let mut second = PublicationAttempt::new(publication_id, 2, Platform::YouTube);
        second.start();
        second.succeed(Some("yt-video-123".to_string()));
        repo.create(&second).await.unwrap();

        let listed = repo.list_for_publication(publication_id).await.unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].attempt_number, 2, "newest first");
        assert_eq!(
            listed[0].remote_operation_id.as_deref(),
            Some("yt-video-123")
        );

        assert_eq!(repo.max_attempt_number(publication_id).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn max_attempt_number_is_zero_when_none_exist() {
        let pool = temp_pool("attempt-repo-empty").await;
        let repo = SqlitePublicationAttemptRepository::new(pool);
        assert_eq!(repo.max_attempt_number(Uuid::new_v4()).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn update_persists_failure_details() {
        let pool = temp_pool("attempt-repo-update").await;
        let publication_id = seed_publication_id(&pool).await;
        let repo = SqlitePublicationAttemptRepository::new(pool);
        let mut attempt = PublicationAttempt::new(publication_id, 1, Platform::TikTok);
        attempt.start();
        repo.create(&attempt).await.unwrap();

        attempt.fail(&crate::domain::publishing::PublishError::NetworkTransient);
        repo.update(&attempt).await.unwrap();

        let reloaded = repo.get(attempt.id).await.unwrap().unwrap();
        assert_eq!(reloaded.status, AttemptStatus::Failed);
        assert_eq!(reloaded.retryable, Some(true));
        assert_eq!(reloaded.error_code.as_deref(), Some("NETWORK_TRANSIENT"));
    }
}
