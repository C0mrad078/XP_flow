use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::platform::Platform;
use crate::domain::platform_account::{ConnectionStatus, PlatformAccount};
use crate::domain::ports::repositories::PlatformAccountRepository;

use super::parse_dt;

pub struct SqlitePlatformAccountRepository {
    pool: SqlitePool,
}

impl SqlitePlatformAccountRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn map_repo_err(err: sqlx::Error) -> DomainError {
    DomainError::Repository(err.to_string())
}

const SELECT_COLUMNS: &str =
    "id, channel_id, platform, display_name, status, external_account_id, \
     connected_at, default_target, created_at, updated_at";

fn row_to_account(row: &sqlx::sqlite::SqliteRow) -> Result<PlatformAccount, DomainError> {
    let connected_at: Option<String> = row.try_get("connected_at").map_err(map_repo_err)?;
    Ok(PlatformAccount {
        id: Uuid::parse_str(&row.try_get::<String, _>("id").map_err(map_repo_err)?)
            .unwrap_or_default(),
        channel_id: Uuid::parse_str(
            &row.try_get::<String, _>("channel_id")
                .map_err(map_repo_err)?,
        )
        .unwrap_or_default(),
        platform: row
            .try_get::<String, _>("platform")
            .map_err(map_repo_err)?
            .parse::<Platform>()
            .map_err(DomainError::Validation)?,
        display_name: row.try_get("display_name").map_err(map_repo_err)?,
        status: row
            .try_get::<String, _>("status")
            .map_err(map_repo_err)?
            .parse::<ConnectionStatus>()
            .map_err(DomainError::Validation)?,
        external_account_id: row.try_get("external_account_id").map_err(map_repo_err)?,
        connected_at: connected_at.map(|s| parse_dt(&s)),
        default_target: row
            .try_get::<i64, _>("default_target")
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
    })
}

#[async_trait]
impl PlatformAccountRepository for SqlitePlatformAccountRepository {
    async fn create(&self, account: &PlatformAccount) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO platform_accounts (id, channel_id, platform, display_name, status, external_account_id, \
             connected_at, default_target, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(account.id.to_string())
        .bind(account.channel_id.to_string())
        .bind(account.platform.as_str())
        .bind(&account.display_name)
        .bind(account.status.as_str())
        .bind(&account.external_account_id)
        .bind(account.connected_at.map(|dt| dt.to_rfc3339()))
        .bind(account.default_target)
        .bind(account.created_at.to_rfc3339())
        .bind(account.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn update(&self, account: &PlatformAccount) -> DomainResult<()> {
        sqlx::query(
            "UPDATE platform_accounts SET display_name = ?, status = ?, external_account_id = ?, \
             connected_at = ?, default_target = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&account.display_name)
        .bind(account.status.as_str())
        .bind(&account.external_account_id)
        .bind(account.connected_at.map(|dt| dt.to_rfc3339()))
        .bind(account.default_target)
        .bind(account.updated_at.to_rfc3339())
        .bind(account.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn get(&self, id: Uuid) -> DomainResult<Option<PlatformAccount>> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM platform_accounts WHERE id = ?"
        ))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_repo_err)?;
        row.as_ref().map(row_to_account).transpose()
    }

    async fn list_for_channel(&self, channel_id: Uuid) -> DomainResult<Vec<PlatformAccount>> {
        let rows = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM platform_accounts WHERE channel_id = ? ORDER BY created_at ASC"
        ))
        .bind(channel_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(map_repo_err)?;
        rows.iter().map(row_to_account).collect()
    }

    async fn get_default_for_channel_platform(
        &self,
        channel_id: Uuid,
        platform: Platform,
    ) -> DomainResult<Option<PlatformAccount>> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM platform_accounts \
             WHERE channel_id = ? AND platform = ? \
             ORDER BY default_target DESC, created_at ASC LIMIT 1"
        ))
        .bind(channel_id.to_string())
        .bind(platform.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_repo_err)?;
        row.as_ref().map(row_to_account).transpose()
    }
}
