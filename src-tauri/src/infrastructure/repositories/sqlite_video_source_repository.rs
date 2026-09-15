use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::ports::repositories::VideoSourceRepository;
use crate::domain::video_source::{VideoSource, VideoSourceType};

use super::parse_dt;

pub struct SqliteVideoSourceRepository {
    pool: SqlitePool,
}

impl SqliteVideoSourceRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn map_repo_err(err: sqlx::Error) -> DomainError {
    DomainError::Repository(err.to_string())
}

const SELECT_COLUMNS: &str =
    "id, workspace_id, name, source_type, folder_path, channel_id, enabled, \
     recursive, watch_enabled, created_at, updated_at, last_scan_at, last_error";

fn row_to_source(row: &sqlx::sqlite::SqliteRow) -> Result<VideoSource, DomainError> {
    let channel_id: Option<String> = row.try_get("channel_id").map_err(map_repo_err)?;
    let last_scan_at: Option<String> = row.try_get("last_scan_at").map_err(map_repo_err)?;

    Ok(VideoSource {
        id: Uuid::parse_str(&row.try_get::<String, _>("id").map_err(map_repo_err)?)
            .unwrap_or_default(),
        workspace_id: Uuid::parse_str(
            &row.try_get::<String, _>("workspace_id")
                .map_err(map_repo_err)?,
        )
        .unwrap_or_default(),
        name: row.try_get("name").map_err(map_repo_err)?,
        source_type: row
            .try_get::<String, _>("source_type")
            .map_err(map_repo_err)?
            .parse::<VideoSourceType>()
            .map_err(DomainError::Validation)?,
        folder_path: row.try_get("folder_path").map_err(map_repo_err)?,
        channel_id: channel_id.and_then(|s| Uuid::parse_str(&s).ok()),
        enabled: row.try_get::<i64, _>("enabled").map_err(map_repo_err)? != 0,
        recursive: row.try_get::<i64, _>("recursive").map_err(map_repo_err)? != 0,
        watch_enabled: row
            .try_get::<i64, _>("watch_enabled")
            .map_err(map_repo_err)?
            != 0,
        created_at: parse_dt(
            &row.try_get::<String, _>("created_at")
                .map_err(map_repo_err)?,
        ),
        updated_at: parse_dt(
            &row.try_get::<String, _>("updated_at")
                .map_err(map_repo_err)?,
        ),
        last_scan_at: last_scan_at.map(|s| parse_dt(&s)),
        last_error: row.try_get("last_error").map_err(map_repo_err)?,
    })
}

#[async_trait]
impl VideoSourceRepository for SqliteVideoSourceRepository {
    async fn create(&self, source: &VideoSource) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO video_sources (id, workspace_id, name, source_type, folder_path, channel_id, enabled, \
             recursive, watch_enabled, created_at, updated_at, last_scan_at, last_error) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(source.id.to_string())
        .bind(source.workspace_id.to_string())
        .bind(&source.name)
        .bind(source.source_type.as_str())
        .bind(&source.folder_path)
        .bind(source.channel_id.map(|id| id.to_string()))
        .bind(source.enabled as i64)
        .bind(source.recursive as i64)
        .bind(source.watch_enabled as i64)
        .bind(source.created_at.to_rfc3339())
        .bind(source.updated_at.to_rfc3339())
        .bind(source.last_scan_at.map(|dt| dt.to_rfc3339()))
        .bind(&source.last_error)
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn update(&self, source: &VideoSource) -> DomainResult<()> {
        sqlx::query(
            "UPDATE video_sources SET name = ?, folder_path = ?, channel_id = ?, enabled = ?, recursive = ?, \
             watch_enabled = ?, updated_at = ?, last_scan_at = ?, last_error = ? WHERE id = ?",
        )
        .bind(&source.name)
        .bind(&source.folder_path)
        .bind(source.channel_id.map(|id| id.to_string()))
        .bind(source.enabled as i64)
        .bind(source.recursive as i64)
        .bind(source.watch_enabled as i64)
        .bind(source.updated_at.to_rfc3339())
        .bind(source.last_scan_at.map(|dt| dt.to_rfc3339()))
        .bind(&source.last_error)
        .bind(source.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn get(&self, id: Uuid) -> DomainResult<Option<VideoSource>> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM video_sources WHERE id = ?"
        ))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_repo_err)?;
        row.as_ref().map(row_to_source).transpose()
    }

    async fn list_for_workspace(&self, workspace_id: Uuid) -> DomainResult<Vec<VideoSource>> {
        let rows = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM video_sources WHERE workspace_id = ? ORDER BY created_at ASC"
        ))
        .bind(workspace_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(map_repo_err)?;
        rows.iter().map(row_to_source).collect()
    }

    async fn delete(&self, id: Uuid) -> DomainResult<()> {
        sqlx::query("DELETE FROM video_sources WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(map_repo_err)?;
        Ok(())
    }
}
