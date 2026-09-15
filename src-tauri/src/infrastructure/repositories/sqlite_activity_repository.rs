use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::activity_event::{ActivityCategory, ActivityEvent, ActivityLevel};
use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::ports::repositories::ActivityRepository;

use super::parse_dt;

pub struct SqliteActivityRepository {
    pool: SqlitePool,
}

impl SqliteActivityRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn map_repo_err(err: sqlx::Error) -> DomainError {
    DomainError::Repository(err.to_string())
}

#[async_trait]
impl ActivityRepository for SqliteActivityRepository {
    async fn record(&self, event: &ActivityEvent) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO activity_events (id, workspace_id, category, level, message, metadata_json, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(event.id.to_string())
        .bind(event.workspace_id.map(|id| id.to_string()))
        .bind(event.category.as_str())
        .bind(event.level.as_str())
        .bind(&event.message)
        .bind(&event.metadata_json)
        .bind(event.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn list_recent(&self, limit: i64) -> DomainResult<Vec<ActivityEvent>> {
        let rows = sqlx::query(
            "SELECT id, workspace_id, category, level, message, metadata_json, created_at
             FROM activity_events ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_repo_err)?;

        rows.iter()
            .map(|row| {
                let workspace_id: Option<String> =
                    row.try_get("workspace_id").map_err(map_repo_err)?;
                Ok(ActivityEvent {
                    id: Uuid::parse_str(&row.try_get::<String, _>("id").map_err(map_repo_err)?)
                        .unwrap_or_default(),
                    workspace_id: workspace_id.and_then(|s| Uuid::parse_str(&s).ok()),
                    category: row
                        .try_get::<String, _>("category")
                        .map_err(map_repo_err)?
                        .parse::<ActivityCategory>()
                        .map_err(DomainError::Validation)?,
                    level: row
                        .try_get::<String, _>("level")
                        .map_err(map_repo_err)?
                        .parse::<ActivityLevel>()
                        .map_err(DomainError::Validation)?,
                    message: row.try_get("message").map_err(map_repo_err)?,
                    metadata_json: row.try_get("metadata_json").map_err(map_repo_err)?,
                    created_at: parse_dt(
                        &row.try_get::<String, _>("created_at")
                            .map_err(map_repo_err)?,
                    ),
                })
            })
            .collect()
    }
}
