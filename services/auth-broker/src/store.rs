use chrono::{DateTime, Duration, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::error::BrokerError;

/// Section 46/41: how long an authorization session stays valid.
pub const SESSION_TTL: Duration = Duration::minutes(10);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionStatus {
    Pending,
    Completed,
    Failed,
    Expired,
}

impl SessionStatus {
    fn as_str(&self) -> &'static str {
        match self {
            SessionStatus::Pending => "pending",
            SessionStatus::Completed => "completed",
            SessionStatus::Failed => "failed",
            SessionStatus::Expired => "expired",
        }
    }

    fn parse(s: &str) -> Self {
        match s {
            "completed" => SessionStatus::Completed,
            "failed" => SessionStatus::Failed,
            "expired" => SessionStatus::Expired,
            _ => SessionStatus::Pending,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OAuthSession {
    pub id: String,
    pub platform: String,
    pub workspace_id: String,
    pub channel_id: String,
    pub state: String,
    pub code_verifier_encrypted: Option<String>,
    pub redirect_uri: Option<String>,
    pub status: SessionStatus,
    pub connection_id: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
}

impl OAuthSession {
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }
}

#[derive(Debug, Clone)]
pub struct Connection {
    pub id: String,
    pub platform: String,
    pub workspace_id: String,
    pub provider_account_id: String,
    pub display_name: Option<String>,
    pub username_or_handle: Option<String>,
    pub avatar_url: Option<String>,
    pub granted_scopes: Vec<String>,
    pub access_token_encrypted: String,
    pub refresh_token_encrypted: Option<String>,
    pub access_expires_at: Option<DateTime<Utc>>,
    pub refresh_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

fn parse_dt(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn opt_dt(value: Option<String>) -> Option<DateTime<Utc>> {
    value.map(|s| parse_dt(&s))
}

pub struct Store {
    pool: SqlitePool,
}

impl Store {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    // --- OAuth sessions ---------------------------------------------

    #[allow(clippy::too_many_arguments)]
    pub async fn create_session(
        &self,
        platform: &str,
        workspace_id: &str,
        channel_id: &str,
        state: &str,
        code_verifier_encrypted: Option<&str>,
        redirect_uri: Option<&str>,
    ) -> Result<OAuthSession, BrokerError> {
        let now = Utc::now();
        let session = OAuthSession {
            id: Uuid::new_v4().to_string(),
            platform: platform.to_string(),
            workspace_id: workspace_id.to_string(),
            channel_id: channel_id.to_string(),
            state: state.to_string(),
            code_verifier_encrypted: code_verifier_encrypted.map(str::to_string),
            redirect_uri: redirect_uri.map(str::to_string),
            status: SessionStatus::Pending,
            connection_id: None,
            error_code: None,
            error_message: None,
            created_at: now,
            expires_at: now + SESSION_TTL,
            consumed_at: None,
        };
        sqlx::query(
            "INSERT INTO oauth_sessions (id, platform, workspace_id, channel_id, state, code_verifier_encrypted, \
             redirect_uri, status, created_at, expires_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&session.id)
        .bind(&session.platform)
        .bind(&session.workspace_id)
        .bind(&session.channel_id)
        .bind(&session.state)
        .bind(&session.code_verifier_encrypted)
        .bind(&session.redirect_uri)
        .bind(session.status.as_str())
        .bind(session.created_at.to_rfc3339())
        .bind(session.expires_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(session)
    }

    /// Creates an already-`Completed` session row directly — TikTok's
    /// exchange path (section 41: desktop supplies its own session id as
    /// an idempotency key; there was no prior broker-side "start" call).
    /// Returns `Err(BrokerError::CodeInvalid)` if a session with this id
    /// already exists (a replayed exchange request, section 26).
    pub async fn create_completed_session(
        &self,
        id: &str,
        platform: &str,
        connection_id: &str,
    ) -> Result<(), BrokerError> {
        let now = Utc::now();
        let result = sqlx::query(
            "INSERT INTO oauth_sessions (id, platform, workspace_id, channel_id, state, status, connection_id, \
             created_at, expires_at, consumed_at) VALUES (?, ?, '', '', '', 'completed', ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(platform)
        .bind(connection_id)
        .bind(now.to_rfc3339())
        .bind((now + SESSION_TTL).to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(ref db_err)) if db_err.is_unique_violation() => {
                Err(BrokerError::CodeInvalid)
            }
            Err(other) => Err(other.into()),
        }
    }

    pub async fn find_session_by_state(
        &self,
        state: &str,
    ) -> Result<Option<OAuthSession>, BrokerError> {
        let row = sqlx::query(
            "SELECT id, platform, workspace_id, channel_id, state, code_verifier_encrypted, redirect_uri, status, \
             connection_id, error_code, error_message, created_at, expires_at, consumed_at \
             FROM oauth_sessions WHERE state = ? ORDER BY created_at DESC LIMIT 1",
        )
        .bind(state)
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(Self::row_to_session).transpose()
    }

    fn row_to_session(row: &sqlx::sqlite::SqliteRow) -> Result<OAuthSession, BrokerError> {
        Ok(OAuthSession {
            id: row.try_get("id")?,
            platform: row.try_get("platform")?,
            workspace_id: row.try_get("workspace_id")?,
            channel_id: row.try_get("channel_id")?,
            state: row.try_get("state")?,
            code_verifier_encrypted: row.try_get("code_verifier_encrypted")?,
            redirect_uri: row.try_get("redirect_uri")?,
            status: SessionStatus::parse(&row.try_get::<String, _>("status")?),
            connection_id: row.try_get("connection_id")?,
            error_code: row.try_get("error_code")?,
            error_message: row.try_get("error_message")?,
            created_at: parse_dt(&row.try_get::<String, _>("created_at")?),
            expires_at: parse_dt(&row.try_get::<String, _>("expires_at")?),
            consumed_at: opt_dt(row.try_get("consumed_at")?),
        })
    }

    pub async fn get_session(&self, id: &str) -> Result<Option<OAuthSession>, BrokerError> {
        let row = sqlx::query(
            "SELECT id, platform, workspace_id, channel_id, state, code_verifier_encrypted, redirect_uri, status, \
             connection_id, error_code, error_message, created_at, expires_at, consumed_at \
             FROM oauth_sessions WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(Self::row_to_session).transpose()
    }

    /// Atomically marks a `Pending` session `Completed` and single-use
    /// (section 12/26) — returns `false` if it was already
    /// consumed/expired/not-pending, so the caller can reject a replay
    /// instead of double-processing it.
    pub async fn try_complete_session(
        &self,
        id: &str,
        connection_id: &str,
    ) -> Result<bool, BrokerError> {
        let result = sqlx::query(
            "UPDATE oauth_sessions SET status = 'completed', connection_id = ?, consumed_at = ? \
             WHERE id = ? AND status = 'pending' AND expires_at > ?",
        )
        .bind(connection_id)
        .bind(Utc::now().to_rfc3339())
        .bind(id)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn mark_session_failed(
        &self,
        id: &str,
        code: &str,
        message: &str,
    ) -> Result<(), BrokerError> {
        sqlx::query(
            "UPDATE oauth_sessions SET status = 'failed', error_code = ?, error_message = ?, consumed_at = ? \
             WHERE id = ? AND status = 'pending'",
        )
        .bind(code)
        .bind(message)
        .bind(Utc::now().to_rfc3339())
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // --- Connections ---------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_connection(
        &self,
        platform: &str,
        workspace_id: &str,
        provider_account_id: &str,
        display_name: Option<&str>,
        username_or_handle: Option<&str>,
        avatar_url: Option<&str>,
        granted_scopes: &[String],
        access_token_encrypted: &str,
        refresh_token_encrypted: Option<&str>,
        access_expires_at: Option<DateTime<Utc>>,
        refresh_expires_at: Option<DateTime<Utc>>,
    ) -> Result<Connection, BrokerError> {
        let existing = self
            .find_connection_by_identity(workspace_id, platform, provider_account_id)
            .await?;
        let now = Utc::now();
        let scopes_json =
            serde_json::to_string(granted_scopes).unwrap_or_else(|_| "[]".to_string());

        let connection = Connection {
            id: existing
                .as_ref()
                .map(|c| c.id.clone())
                .unwrap_or_else(|| Uuid::new_v4().to_string()),
            platform: platform.to_string(),
            workspace_id: workspace_id.to_string(),
            provider_account_id: provider_account_id.to_string(),
            display_name: display_name.map(str::to_string),
            username_or_handle: username_or_handle.map(str::to_string),
            avatar_url: avatar_url.map(str::to_string),
            granted_scopes: granted_scopes.to_vec(),
            access_token_encrypted: access_token_encrypted.to_string(),
            refresh_token_encrypted: refresh_token_encrypted.map(str::to_string),
            access_expires_at,
            refresh_expires_at,
            created_at: existing.as_ref().map(|c| c.created_at).unwrap_or(now),
            updated_at: now,
            revoked_at: None,
        };

        sqlx::query(
            "INSERT INTO connections (id, platform, workspace_id, provider_account_id, display_name, \
             username_or_handle, avatar_url, granted_scopes_json, access_token_encrypted, refresh_token_encrypted, \
             access_expires_at, refresh_expires_at, created_at, updated_at, revoked_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL) \
             ON CONFLICT(id) DO UPDATE SET display_name = excluded.display_name, \
             username_or_handle = excluded.username_or_handle, avatar_url = excluded.avatar_url, \
             granted_scopes_json = excluded.granted_scopes_json, access_token_encrypted = excluded.access_token_encrypted, \
             refresh_token_encrypted = excluded.refresh_token_encrypted, access_expires_at = excluded.access_expires_at, \
             refresh_expires_at = excluded.refresh_expires_at, updated_at = excluded.updated_at, revoked_at = NULL",
        )
        .bind(&connection.id)
        .bind(&connection.platform)
        .bind(&connection.workspace_id)
        .bind(&connection.provider_account_id)
        .bind(&connection.display_name)
        .bind(&connection.username_or_handle)
        .bind(&connection.avatar_url)
        .bind(&scopes_json)
        .bind(&connection.access_token_encrypted)
        .bind(&connection.refresh_token_encrypted)
        .bind(connection.access_expires_at.map(|d| d.to_rfc3339()))
        .bind(connection.refresh_expires_at.map(|d| d.to_rfc3339()))
        .bind(connection.created_at.to_rfc3339())
        .bind(connection.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(connection)
    }

    pub async fn get_connection(&self, id: &str) -> Result<Option<Connection>, BrokerError> {
        let row = sqlx::query(&format!("{SELECT_CONNECTION} WHERE id = ?"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(row_to_connection).transpose()
    }

    pub async fn find_connection_by_identity(
        &self,
        workspace_id: &str,
        platform: &str,
        provider_account_id: &str,
    ) -> Result<Option<Connection>, BrokerError> {
        let row = sqlx::query(&format!(
            "{SELECT_CONNECTION} WHERE workspace_id = ? AND platform = ? AND provider_account_id = ? AND revoked_at IS NULL"
        ))
        .bind(workspace_id)
        .bind(platform)
        .bind(provider_account_id)
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(row_to_connection).transpose()
    }

    pub async fn update_connection_tokens(
        &self,
        id: &str,
        access_token_encrypted: &str,
        refresh_token_encrypted: Option<&str>,
        access_expires_at: Option<DateTime<Utc>>,
        refresh_expires_at: Option<DateTime<Utc>>,
    ) -> Result<(), BrokerError> {
        sqlx::query(
            "UPDATE connections SET access_token_encrypted = ?, refresh_token_encrypted = COALESCE(?, refresh_token_encrypted), \
             access_expires_at = ?, refresh_expires_at = ?, updated_at = ? WHERE id = ?",
        )
        .bind(access_token_encrypted)
        .bind(refresh_token_encrypted)
        .bind(access_expires_at.map(|d| d.to_rfc3339()))
        .bind(refresh_expires_at.map(|d| d.to_rfc3339()))
        .bind(Utc::now().to_rfc3339())
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn revoke_connection(&self, id: &str) -> Result<(), BrokerError> {
        sqlx::query("UPDATE connections SET revoked_at = ?, updated_at = ? WHERE id = ?")
            .bind(Utc::now().to_rfc3339())
            .bind(Utc::now().to_rfc3339())
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

const SELECT_CONNECTION: &str = "SELECT id, platform, workspace_id, provider_account_id, display_name, \
     username_or_handle, avatar_url, granted_scopes_json, access_token_encrypted, refresh_token_encrypted, \
     access_expires_at, refresh_expires_at, created_at, updated_at, revoked_at FROM connections";

fn row_to_connection(row: &sqlx::sqlite::SqliteRow) -> Result<Connection, BrokerError> {
    let scopes_json: String = row.try_get("granted_scopes_json")?;
    Ok(Connection {
        id: row.try_get("id")?,
        platform: row.try_get("platform")?,
        workspace_id: row.try_get("workspace_id")?,
        provider_account_id: row.try_get("provider_account_id")?,
        display_name: row.try_get("display_name")?,
        username_or_handle: row.try_get("username_or_handle")?,
        avatar_url: row.try_get("avatar_url")?,
        granted_scopes: serde_json::from_str(&scopes_json).unwrap_or_default(),
        access_token_encrypted: row.try_get("access_token_encrypted")?,
        refresh_token_encrypted: row.try_get("refresh_token_encrypted")?,
        access_expires_at: opt_dt(row.try_get("access_expires_at")?),
        refresh_expires_at: opt_dt(row.try_get("refresh_expires_at")?),
        created_at: parse_dt(&row.try_get::<String, _>("created_at")?),
        updated_at: parse_dt(&row.try_get::<String, _>("updated_at")?),
        revoked_at: opt_dt(row.try_get("revoked_at")?),
    })
}
