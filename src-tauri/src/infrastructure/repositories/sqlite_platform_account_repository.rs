use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::capability::Capability;
use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::platform::Platform;
use crate::domain::platform_account::{PlatformAccount, PlatformAccountStatus};
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

const SELECT_COLUMNS: &str = "id, workspace_id, channel_id, platform, provider_account_id, provider_connection_id, \
     display_name, username_or_handle, avatar_url, status, granted_scopes_json, capabilities_json, default_target, \
     access_expires_at, refresh_expires_at, connected_at, last_validated_at, last_refreshed_at, last_error_code, \
     last_error_message, created_at, updated_at";

fn opt_dt(value: Option<String>) -> Option<DateTime<Utc>> {
    value.map(|s| parse_dt(&s))
}

fn row_to_account(row: &sqlx::sqlite::SqliteRow) -> Result<PlatformAccount, DomainError> {
    let granted_scopes_json: String = row.try_get("granted_scopes_json").map_err(map_repo_err)?;
    let capabilities_json: String = row.try_get("capabilities_json").map_err(map_repo_err)?;
    let capabilities: Vec<String> = serde_json::from_str(&capabilities_json).unwrap_or_default();

    Ok(PlatformAccount {
        id: Uuid::parse_str(&row.try_get::<String, _>("id").map_err(map_repo_err)?)
            .unwrap_or_default(),
        workspace_id: Uuid::parse_str(
            &row.try_get::<String, _>("workspace_id")
                .map_err(map_repo_err)?,
        )
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
        provider_account_id: row.try_get("provider_account_id").map_err(map_repo_err)?,
        provider_connection_id: row
            .try_get("provider_connection_id")
            .map_err(map_repo_err)?,
        display_name: row.try_get("display_name").map_err(map_repo_err)?,
        username_or_handle: row.try_get("username_or_handle").map_err(map_repo_err)?,
        avatar_url: row.try_get("avatar_url").map_err(map_repo_err)?,
        status: row
            .try_get::<String, _>("status")
            .map_err(map_repo_err)?
            .parse::<PlatformAccountStatus>()
            .map_err(DomainError::Validation)?,
        granted_scopes: serde_json::from_str(&granted_scopes_json).unwrap_or_default(),
        capabilities: capabilities
            .iter()
            .filter_map(|s| s.parse::<Capability>().ok())
            .collect(),
        default_target: row
            .try_get::<i64, _>("default_target")
            .map_err(map_repo_err)?
            != 0,
        access_expires_at: opt_dt(row.try_get("access_expires_at").map_err(map_repo_err)?),
        refresh_expires_at: opt_dt(row.try_get("refresh_expires_at").map_err(map_repo_err)?),
        connected_at: opt_dt(row.try_get("connected_at").map_err(map_repo_err)?),
        last_validated_at: opt_dt(row.try_get("last_validated_at").map_err(map_repo_err)?),
        last_refreshed_at: opt_dt(row.try_get("last_refreshed_at").map_err(map_repo_err)?),
        last_error_code: row.try_get("last_error_code").map_err(map_repo_err)?,
        last_error_message: row.try_get("last_error_message").map_err(map_repo_err)?,
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
            "INSERT INTO platform_accounts (id, workspace_id, channel_id, platform, provider_account_id, \
             provider_connection_id, display_name, username_or_handle, avatar_url, status, granted_scopes_json, \
             capabilities_json, default_target, access_expires_at, refresh_expires_at, connected_at, \
             last_validated_at, last_refreshed_at, last_error_code, last_error_message, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(account.id.to_string())
        .bind(account.workspace_id.to_string())
        .bind(account.channel_id.to_string())
        .bind(account.platform.as_str())
        .bind(&account.provider_account_id)
        .bind(&account.provider_connection_id)
        .bind(&account.display_name)
        .bind(&account.username_or_handle)
        .bind(&account.avatar_url)
        .bind(account.status.as_str())
        .bind(serde_json::to_string(&account.granted_scopes).unwrap_or_else(|_| "[]".to_string()))
        .bind(
            serde_json::to_string(&account.capabilities.iter().map(|c| c.as_str()).collect::<Vec<_>>())
                .unwrap_or_else(|_| "[]".to_string()),
        )
        .bind(account.default_target)
        .bind(account.access_expires_at.map(|dt| dt.to_rfc3339()))
        .bind(account.refresh_expires_at.map(|dt| dt.to_rfc3339()))
        .bind(account.connected_at.map(|dt| dt.to_rfc3339()))
        .bind(account.last_validated_at.map(|dt| dt.to_rfc3339()))
        .bind(account.last_refreshed_at.map(|dt| dt.to_rfc3339()))
        .bind(&account.last_error_code)
        .bind(&account.last_error_message)
        .bind(account.created_at.to_rfc3339())
        .bind(account.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|err| match err {
            sqlx::Error::Database(ref db_err) if db_err.is_unique_violation() => DomainError::Conflict(
                "an account for this provider identity is already connected in this workspace".to_string(),
            ),
            other => map_repo_err(other),
        })?;
        Ok(())
    }

    async fn update(&self, account: &PlatformAccount) -> DomainResult<()> {
        sqlx::query(
            "UPDATE platform_accounts SET provider_account_id = ?, provider_connection_id = ?, display_name = ?, \
             username_or_handle = ?, avatar_url = ?, status = ?, granted_scopes_json = ?, capabilities_json = ?, \
             default_target = ?, access_expires_at = ?, refresh_expires_at = ?, connected_at = ?, \
             last_validated_at = ?, last_refreshed_at = ?, last_error_code = ?, last_error_message = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(&account.provider_account_id)
        .bind(&account.provider_connection_id)
        .bind(&account.display_name)
        .bind(&account.username_or_handle)
        .bind(&account.avatar_url)
        .bind(account.status.as_str())
        .bind(serde_json::to_string(&account.granted_scopes).unwrap_or_else(|_| "[]".to_string()))
        .bind(
            serde_json::to_string(&account.capabilities.iter().map(|c| c.as_str()).collect::<Vec<_>>())
                .unwrap_or_else(|_| "[]".to_string()),
        )
        .bind(account.default_target)
        .bind(account.access_expires_at.map(|dt| dt.to_rfc3339()))
        .bind(account.refresh_expires_at.map(|dt| dt.to_rfc3339()))
        .bind(account.connected_at.map(|dt| dt.to_rfc3339()))
        .bind(account.last_validated_at.map(|dt| dt.to_rfc3339()))
        .bind(account.last_refreshed_at.map(|dt| dt.to_rfc3339()))
        .bind(&account.last_error_code)
        .bind(&account.last_error_message)
        .bind(account.updated_at.to_rfc3339())
        .bind(account.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|err| match err {
            sqlx::Error::Database(ref db_err) if db_err.is_unique_violation() => DomainError::Conflict(
                "an account for this provider identity is already connected in this workspace".to_string(),
            ),
            other => map_repo_err(other),
        })?;
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

    async fn list_for_workspace(&self, workspace_id: Uuid) -> DomainResult<Vec<PlatformAccount>> {
        let rows = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM platform_accounts WHERE workspace_id = ? ORDER BY created_at ASC"
        ))
        .bind(workspace_id.to_string())
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

    async fn find_by_provider_identity(
        &self,
        workspace_id: Uuid,
        platform: Platform,
        provider_account_id: &str,
    ) -> DomainResult<Option<PlatformAccount>> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM platform_accounts \
             WHERE workspace_id = ? AND platform = ? AND provider_account_id = ? LIMIT 1"
        ))
        .bind(workspace_id.to_string())
        .bind(platform.as_str())
        .bind(provider_account_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_repo_err)?;
        row.as_ref().map(row_to_account).transpose()
    }

    async fn list_due_for_refresh(
        &self,
        before: DateTime<Utc>,
    ) -> DomainResult<Vec<PlatformAccount>> {
        let rows = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM platform_accounts \
             WHERE status = 'connected' AND access_expires_at IS NOT NULL AND access_expires_at <= ? \
             ORDER BY access_expires_at ASC"
        ))
        .bind(before.to_rfc3339())
        .fetch_all(&self.pool)
        .await
        .map_err(map_repo_err)?;
        rows.iter().map(row_to_account).collect()
    }
}
