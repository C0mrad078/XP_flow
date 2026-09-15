use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::ports::repositories::QueueItemRepository;
use crate::domain::queue_item::QueueItem;

use super::parse_dt;

pub struct SqliteQueueItemRepository {
    pool: SqlitePool,
}

impl SqliteQueueItemRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn map_repo_err(err: sqlx::Error) -> DomainError {
    DomainError::Repository(err.to_string())
}

const SELECT_COLUMNS: &str = "id, workspace_id, publication_id, position, created_at, updated_at";

fn row_to_item(row: &sqlx::sqlite::SqliteRow) -> Result<QueueItem, DomainError> {
    Ok(QueueItem {
        id: Uuid::parse_str(&row.try_get::<String, _>("id").map_err(map_repo_err)?)
            .unwrap_or_default(),
        workspace_id: Uuid::parse_str(
            &row.try_get::<String, _>("workspace_id")
                .map_err(map_repo_err)?,
        )
        .unwrap_or_default(),
        publication_id: Uuid::parse_str(
            &row.try_get::<String, _>("publication_id")
                .map_err(map_repo_err)?,
        )
        .unwrap_or_default(),
        position: row.try_get("position").map_err(map_repo_err)?,
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
impl QueueItemRepository for SqliteQueueItemRepository {
    async fn create(&self, item: &QueueItem) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO queue_items (id, workspace_id, publication_id, position, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(item.id.to_string())
        .bind(item.workspace_id.to_string())
        .bind(item.publication_id.to_string())
        .bind(item.position)
        .bind(item.created_at.to_rfc3339())
        .bind(item.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn update(&self, item: &QueueItem) -> DomainResult<()> {
        sqlx::query("UPDATE queue_items SET position = ?, updated_at = ? WHERE id = ?")
            .bind(item.position)
            .bind(item.updated_at.to_rfc3339())
            .bind(item.id.to_string())
            .execute(&self.pool)
            .await
            .map_err(map_repo_err)?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> DomainResult<()> {
        sqlx::query("DELETE FROM queue_items WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(map_repo_err)?;
        Ok(())
    }

    async fn delete_for_publication(&self, publication_id: Uuid) -> DomainResult<()> {
        sqlx::query("DELETE FROM queue_items WHERE publication_id = ?")
            .bind(publication_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(map_repo_err)?;
        Ok(())
    }

    async fn get_for_publication(&self, publication_id: Uuid) -> DomainResult<Option<QueueItem>> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM queue_items WHERE publication_id = ?"
        ))
        .bind(publication_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_repo_err)?;
        row.as_ref().map(row_to_item).transpose()
    }

    async fn list_unscheduled_for_workspace(
        &self,
        workspace_id: Uuid,
    ) -> DomainResult<Vec<QueueItem>> {
        let rows = sqlx::query(&format!(
            "SELECT qi.id, qi.workspace_id, qi.publication_id, qi.position, qi.created_at, qi.updated_at \
             FROM queue_items qi \
             JOIN publications p ON p.id = qi.publication_id \
             WHERE qi.workspace_id = ? AND p.scheduled_at IS NULL \
             ORDER BY qi.position ASC"
        ))
        .bind(workspace_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(map_repo_err)?;
        rows.iter().map(row_to_item).collect()
    }

    async fn next_position(&self, workspace_id: Uuid) -> DomainResult<i64> {
        let row = sqlx::query(
            "SELECT COALESCE(MAX(position), -1) + 1 AS next_position FROM queue_items WHERE workspace_id = ?",
        )
        .bind(workspace_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(map_repo_err)?;
        row.try_get("next_position").map_err(map_repo_err)
    }
}
