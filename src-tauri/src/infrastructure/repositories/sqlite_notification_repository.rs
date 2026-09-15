use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::notification::{Notification, NotificationType};
use crate::domain::ports::repositories::NotificationRepository;

use super::parse_dt;

pub struct SqliteNotificationRepository {
    pool: SqlitePool,
}

impl SqliteNotificationRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn map_repo_err(err: sqlx::Error) -> DomainError {
    DomainError::Repository(err.to_string())
}

#[async_trait]
impl NotificationRepository for SqliteNotificationRepository {
    async fn create(&self, notification: &Notification) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO notifications (id, workspace_id, type, title, message, read, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(notification.id.to_string())
        .bind(notification.workspace_id.map(|id| id.to_string()))
        .bind(notification.notification_type.as_str())
        .bind(&notification.title)
        .bind(&notification.message)
        .bind(notification.read)
        .bind(notification.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn list_recent(&self, limit: i64) -> DomainResult<Vec<Notification>> {
        let rows = sqlx::query(
            "SELECT id, workspace_id, type, title, message, read, created_at
             FROM notifications ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_repo_err)?;

        rows.iter()
            .map(|row| {
                let workspace_id: Option<String> =
                    row.try_get("workspace_id").map_err(map_repo_err)?;
                Ok(Notification {
                    id: Uuid::parse_str(&row.try_get::<String, _>("id").map_err(map_repo_err)?)
                        .unwrap_or_default(),
                    workspace_id: workspace_id.and_then(|s| Uuid::parse_str(&s).ok()),
                    notification_type: row
                        .try_get::<String, _>("type")
                        .map_err(map_repo_err)?
                        .parse::<NotificationType>()
                        .map_err(DomainError::Validation)?,
                    title: row.try_get("title").map_err(map_repo_err)?,
                    message: row.try_get("message").map_err(map_repo_err)?,
                    read: row.try_get("read").map_err(map_repo_err)?,
                    created_at: parse_dt(
                        &row.try_get::<String, _>("created_at")
                            .map_err(map_repo_err)?,
                    ),
                })
            })
            .collect()
    }

    async fn unread_count(&self) -> DomainResult<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM notifications WHERE read = 0")
            .fetch_one(&self.pool)
            .await
            .map_err(map_repo_err)?;
        row.try_get("count").map_err(map_repo_err)
    }

    async fn mark_read(&self, id: Uuid) -> DomainResult<()> {
        sqlx::query("UPDATE notifications SET read = 1 WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(map_repo_err)?;
        Ok(())
    }
}
