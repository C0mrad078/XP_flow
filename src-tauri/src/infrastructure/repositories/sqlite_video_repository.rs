use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::ports::repositories::VideoRepository;
use crate::domain::video::Video;

use super::parse_dt;

pub struct SqliteVideoRepository {
    pool: SqlitePool,
}

impl SqliteVideoRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn map_repo_err(err: sqlx::Error) -> DomainError {
    DomainError::Repository(err.to_string())
}

#[async_trait]
impl VideoRepository for SqliteVideoRepository {
    async fn create(&self, video: &Video) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO videos (id, workspace_id, title, duration_seconds, file_path, file_size_bytes, checksum, imported_at, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(video.id.to_string())
        .bind(video.workspace_id.to_string())
        .bind(&video.title)
        .bind(video.duration_seconds)
        .bind(&video.file_path)
        .bind(video.file_size_bytes)
        .bind(&video.checksum)
        .bind(video.imported_at.to_rfc3339())
        .bind(video.created_at.to_rfc3339())
        .bind(video.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn list_for_workspace(&self, workspace_id: Uuid) -> DomainResult<Vec<Video>> {
        let rows = sqlx::query(
            "SELECT id, workspace_id, title, duration_seconds, file_path, file_size_bytes, checksum, imported_at, created_at, updated_at
             FROM videos WHERE workspace_id = ? ORDER BY imported_at DESC",
        )
        .bind(workspace_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(map_repo_err)?;

        rows.iter()
            .map(|row| {
                Ok(Video {
                    id: Uuid::parse_str(&row.try_get::<String, _>("id").map_err(map_repo_err)?)
                        .unwrap_or_default(),
                    workspace_id: Uuid::parse_str(
                        &row.try_get::<String, _>("workspace_id")
                            .map_err(map_repo_err)?,
                    )
                    .unwrap_or_default(),
                    title: row.try_get("title").map_err(map_repo_err)?,
                    duration_seconds: row.try_get("duration_seconds").map_err(map_repo_err)?,
                    file_path: row.try_get("file_path").map_err(map_repo_err)?,
                    file_size_bytes: row.try_get("file_size_bytes").map_err(map_repo_err)?,
                    checksum: row.try_get("checksum").map_err(map_repo_err)?,
                    imported_at: parse_dt(
                        &row.try_get::<String, _>("imported_at")
                            .map_err(map_repo_err)?,
                    ),
                    created_at: parse_dt(
                        &row.try_get::<String, _>("created_at")
                            .map_err(map_repo_err)?,
                    ),
                    updated_at: parse_dt(
                        &row.try_get::<String, _>("updated_at")
                            .map_err(map_repo_err)?,
                    ),
                })
            })
            .collect()
    }
}
