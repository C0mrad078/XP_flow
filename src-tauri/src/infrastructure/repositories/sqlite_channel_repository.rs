use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::channel::{Channel, ChannelStatus};
use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::ports::repositories::ChannelRepository;

use super::parse_dt;

pub struct SqliteChannelRepository {
    pool: SqlitePool,
}

impl SqliteChannelRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn map_repo_err(err: sqlx::Error) -> DomainError {
    DomainError::Repository(err.to_string())
}

const SELECT_COLUMNS: &str =
    "id, workspace_id, name, niche, description, status, created_at, updated_at";

fn row_to_channel(row: &sqlx::sqlite::SqliteRow) -> Result<Channel, DomainError> {
    Ok(Channel {
        id: Uuid::parse_str(&row.try_get::<String, _>("id").map_err(map_repo_err)?)
            .unwrap_or_default(),
        workspace_id: Uuid::parse_str(
            &row.try_get::<String, _>("workspace_id")
                .map_err(map_repo_err)?,
        )
        .unwrap_or_default(),
        name: row.try_get("name").map_err(map_repo_err)?,
        niche: row.try_get("niche").map_err(map_repo_err)?,
        description: row.try_get("description").map_err(map_repo_err)?,
        status: row
            .try_get::<String, _>("status")
            .map_err(map_repo_err)?
            .parse::<ChannelStatus>()
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
impl ChannelRepository for SqliteChannelRepository {
    async fn create(&self, channel: &Channel) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO channels (id, workspace_id, name, niche, description, status, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(channel.id.to_string())
        .bind(channel.workspace_id.to_string())
        .bind(&channel.name)
        .bind(&channel.niche)
        .bind(&channel.description)
        .bind(channel.status.as_str())
        .bind(channel.created_at.to_rfc3339())
        .bind(channel.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn update(&self, channel: &Channel) -> DomainResult<()> {
        sqlx::query(
            "UPDATE channels SET name = ?, niche = ?, description = ?, status = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&channel.name)
        .bind(&channel.niche)
        .bind(&channel.description)
        .bind(channel.status.as_str())
        .bind(channel.updated_at.to_rfc3339())
        .bind(channel.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn get(&self, id: Uuid) -> DomainResult<Option<Channel>> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM channels WHERE id = ?"
        ))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_repo_err)?;
        row.as_ref().map(row_to_channel).transpose()
    }

    async fn list_for_workspace(&self, workspace_id: Uuid) -> DomainResult<Vec<Channel>> {
        let rows = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM channels WHERE workspace_id = ? ORDER BY created_at ASC"
        ))
        .bind(workspace_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(map_repo_err)?;

        rows.iter().map(row_to_channel).collect()
    }
}
