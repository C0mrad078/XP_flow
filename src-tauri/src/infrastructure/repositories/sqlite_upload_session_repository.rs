use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::platform::Platform;
use crate::domain::ports::repositories::UploadSessionRepository;
use crate::domain::publishing::{RemoteUploadState, SessionType, UploadSession};

use super::parse_dt;

pub struct SqliteUploadSessionRepository {
    pool: SqlitePool,
}

impl SqliteUploadSessionRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn map_repo_err(err: sqlx::Error) -> DomainError {
    DomainError::Repository(err.to_string())
}

const SELECT_COLUMNS: &str = "id, publication_id, attempt_id, provider, session_type, remote_session_id, \
     remote_upload_url, remote_publish_id, remote_upload_token, bytes_total, bytes_committed, expires_at, \
     state, created_at, updated_at";

fn row_to_session(row: &sqlx::sqlite::SqliteRow) -> Result<UploadSession, DomainError> {
    let expires_at: Option<String> = row.try_get("expires_at").map_err(map_repo_err)?;

    Ok(UploadSession {
        id: Uuid::parse_str(&row.try_get::<String, _>("id").map_err(map_repo_err)?)
            .unwrap_or_default(),
        publication_id: Uuid::parse_str(
            &row.try_get::<String, _>("publication_id")
                .map_err(map_repo_err)?,
        )
        .unwrap_or_default(),
        attempt_id: Uuid::parse_str(
            &row.try_get::<String, _>("attempt_id")
                .map_err(map_repo_err)?,
        )
        .unwrap_or_default(),
        provider: row
            .try_get::<String, _>("provider")
            .map_err(map_repo_err)?
            .parse::<Platform>()
            .map_err(DomainError::Validation)?,
        session_type: row
            .try_get::<String, _>("session_type")
            .map_err(map_repo_err)?
            .parse::<SessionType>()
            .map_err(DomainError::Validation)?,
        remote_session_id: row.try_get("remote_session_id").map_err(map_repo_err)?,
        remote_upload_url: row.try_get("remote_upload_url").map_err(map_repo_err)?,
        remote_publish_id: row.try_get("remote_publish_id").map_err(map_repo_err)?,
        remote_upload_token: row.try_get("remote_upload_token").map_err(map_repo_err)?,
        bytes_total: row.try_get("bytes_total").map_err(map_repo_err)?,
        bytes_committed: row.try_get("bytes_committed").map_err(map_repo_err)?,
        expires_at: expires_at.map(|s| parse_dt(&s)),
        state: row
            .try_get::<String, _>("state")
            .map_err(map_repo_err)?
            .parse::<RemoteUploadState>()
            .map_err(DomainError::Validation)?,
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

#[async_trait]
impl UploadSessionRepository for SqliteUploadSessionRepository {
    async fn checkpoint_bytes(&self, id: Uuid, acknowledged: i64) -> DomainResult<()> {
        sqlx::query("UPDATE upload_sessions SET bytes_committed = MAX(bytes_committed, ?), updated_at = ? WHERE id = ?")
            .bind(acknowledged.max(0)).bind(chrono::Utc::now().to_rfc3339())
            .bind(id.to_string()).execute(&self.pool).await.map_err(map_repo_err)?;
        Ok(())
    }
    async fn create(&self, session: &UploadSession) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO upload_sessions (id, publication_id, attempt_id, provider, session_type, remote_session_id, \
             remote_upload_url, remote_publish_id, remote_upload_token, bytes_total, bytes_committed, expires_at, \
             state, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(session.id.to_string())
        .bind(session.publication_id.to_string())
        .bind(session.attempt_id.to_string())
        .bind(session.provider.as_str())
        .bind(session.session_type.as_str())
        .bind(&session.remote_session_id)
        .bind(&session.remote_upload_url)
        .bind(&session.remote_publish_id)
        .bind(&session.remote_upload_token)
        .bind(session.bytes_total)
        .bind(session.bytes_committed)
        .bind(session.expires_at.map(|dt| dt.to_rfc3339()))
        .bind(session.state.as_str())
        .bind(session.created_at.to_rfc3339())
        .bind(session.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn update(&self, session: &UploadSession) -> DomainResult<()> {
        sqlx::query(
            "UPDATE upload_sessions SET remote_session_id = ?, remote_upload_url = ?, remote_publish_id = ?, \
             remote_upload_token = ?, bytes_total = ?, bytes_committed = MAX(bytes_committed, ?), expires_at = ?, state = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(&session.remote_session_id)
        .bind(&session.remote_upload_url)
        .bind(&session.remote_publish_id)
        .bind(&session.remote_upload_token)
        .bind(session.bytes_total)
        .bind(session.bytes_committed)
        .bind(session.expires_at.map(|dt| dt.to_rfc3339()))
        .bind(session.state.as_str())
        .bind(session.updated_at.to_rfc3339())
        .bind(session.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn get(&self, id: Uuid) -> DomainResult<Option<UploadSession>> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM upload_sessions WHERE id = ?"
        ))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_repo_err)?;
        row.as_ref().map(row_to_session).transpose()
    }

    async fn latest_for_publication(
        &self,
        publication_id: Uuid,
    ) -> DomainResult<Option<UploadSession>> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM upload_sessions WHERE publication_id = ? ORDER BY created_at DESC LIMIT 1"
        ))
        .bind(publication_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_repo_err)?;
        row.as_ref().map(row_to_session).transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::publishing::PublicationAttempt;
    use crate::infrastructure::repositories::SqlitePublicationAttemptRepository;
    use crate::test_support::{
        seed_channel, seed_publication, seed_video, seed_workspace_and_source, temp_pool,
    };

    async fn seed_publication_and_attempt(pool: &SqlitePool) -> (Uuid, Uuid) {
        let (workspace_id, source_id) = seed_workspace_and_source(pool).await;
        let channel_id = seed_channel(pool, workspace_id, "Channel").await;
        let video_id = seed_video(pool, workspace_id, source_id, Some(channel_id), "video").await;
        let publication_id = seed_publication(pool, workspace_id, channel_id, video_id).await;

        let attempt = PublicationAttempt::new(publication_id, 1, Platform::TikTok);
        let attempt_repo = SqlitePublicationAttemptRepository::new(pool.clone());
        crate::domain::ports::repositories::PublicationAttemptRepository::create(
            &attempt_repo,
            &attempt,
        )
        .await
        .unwrap();
        (publication_id, attempt.id)
    }

    #[tokio::test]
    async fn create_then_update_round_trips_and_redacts_nothing_in_storage() {
        let pool = temp_pool("upload-session-repo").await;
        let (publication_id, attempt_id) = seed_publication_and_attempt(&pool).await;
        let repo = SqliteUploadSessionRepository::new(pool);

        let mut session = UploadSession::new(
            publication_id,
            attempt_id,
            Platform::TikTok,
            SessionType::DirectPost,
        );
        session.remote_upload_url = Some("https://upload.example/secret".to_string());
        session.bytes_total = Some(1_000_000);
        repo.create(&session).await.unwrap();

        session.bytes_committed = 500_000;
        session.state = RemoteUploadState::Transferring;
        repo.update(&session).await.unwrap();

        let reloaded = repo.get(session.id).await.unwrap().unwrap();
        assert_eq!(reloaded.bytes_committed, 500_000);
        assert_eq!(reloaded.state, RemoteUploadState::Transferring);
        assert_eq!(
            reloaded.remote_upload_url.as_deref(),
            Some("https://upload.example/secret")
        );

        let latest = repo
            .latest_for_publication(publication_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(latest.id, session.id);
    }

    #[tokio::test]
    async fn latest_for_publication_is_none_when_no_session_exists() {
        let pool = temp_pool("upload-session-repo-empty").await;
        let repo = SqliteUploadSessionRepository::new(pool);
        assert!(repo
            .latest_for_publication(Uuid::new_v4())
            .await
            .unwrap()
            .is_none());
    }
}
