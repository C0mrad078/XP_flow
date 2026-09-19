use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::analytics::{
    AnalyticsAvailability, AnalyticsSyncState, ChannelMetricSnapshot, PublicationMetricSnapshot,
};
use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::platform::Platform;
use crate::domain::ports::repositories::AnalyticsRepository;

pub struct SqliteAnalyticsRepository {
    pool: SqlitePool,
}
impl SqliteAnalyticsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}
fn err(e: sqlx::Error) -> DomainError {
    DomainError::Repository(e.to_string())
}
fn dt(s: String) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(&s)
        .map(|v| v.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}
fn availability(s: String) -> AnalyticsAvailability {
    serde_json::from_str(&format!("\"{s}\"")).unwrap_or(AnalyticsAvailability::Failed)
}

#[async_trait]
impl AnalyticsRepository for SqliteAnalyticsRepository {
    async fn insert_publication_snapshot(&self, s: &PublicationMetricSnapshot) -> DomainResult<()> {
        sqlx::query("INSERT INTO publication_metric_snapshots (id, publication_id, provider, captured_at, views, likes, comments, shares, availability, error_code) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(s.id.to_string()).bind(s.publication_id.to_string()).bind(s.provider.as_str()).bind(s.captured_at.to_rfc3339())
            .bind(s.views).bind(s.likes).bind(s.comments).bind(s.shares)
            .bind(serde_json::to_string(&s.availability).unwrap_or_else(|_| "\"failed\"".into()).trim_matches('"'))
            .bind(&s.error_code).execute(&self.pool).await.map_err(err)?;
        Ok(())
    }
    async fn insert_channel_snapshot(&self, s: &ChannelMetricSnapshot) -> DomainResult<()> {
        sqlx::query("INSERT INTO channel_metric_snapshots (id, channel_id, provider, captured_at, followers, total_views, availability, error_code) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(s.id.to_string()).bind(s.channel_id.to_string()).bind(s.provider.as_str()).bind(s.captured_at.to_rfc3339())
            .bind(s.followers).bind(s.total_views)
            .bind(serde_json::to_string(&s.availability).unwrap_or_else(|_| "\"failed\"".into()).trim_matches('"'))
            .bind(&s.error_code).execute(&self.pool).await.map_err(err)?;
        Ok(())
    }
    async fn list_publication_snapshots(
        &self,
        id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> DomainResult<Vec<PublicationMetricSnapshot>> {
        let rows = sqlx::query("SELECT id, publication_id, provider, captured_at, views, likes, comments, shares, availability, error_code FROM publication_metric_snapshots WHERE publication_id = ? AND captured_at >= ? AND captured_at < ? ORDER BY captured_at ASC LIMIT 1000")
            .bind(id.to_string()).bind(from.to_rfc3339()).bind(to.to_rfc3339()).fetch_all(&self.pool).await.map_err(err)?;
        rows.iter()
            .map(|r| {
                Ok(PublicationMetricSnapshot {
                    id: Uuid::parse_str(&r.try_get::<String, _>("id").map_err(err)?)
                        .unwrap_or_default(),
                    publication_id: id,
                    provider: r
                        .try_get::<String, _>("provider")
                        .map_err(err)?
                        .parse::<Platform>()
                        .map_err(DomainError::Validation)?,
                    captured_at: dt(r.try_get("captured_at").map_err(err)?),
                    views: r.try_get("views").map_err(err)?,
                    likes: r.try_get("likes").map_err(err)?,
                    comments: r.try_get("comments").map_err(err)?,
                    shares: r.try_get("shares").map_err(err)?,
                    availability: availability(r.try_get("availability").map_err(err)?),
                    error_code: r.try_get("error_code").map_err(err)?,
                })
            })
            .collect()
    }
    async fn list_channel_snapshots(
        &self,
        id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> DomainResult<Vec<ChannelMetricSnapshot>> {
        let rows = sqlx::query("SELECT id, channel_id, provider, captured_at, followers, total_views, availability, error_code FROM channel_metric_snapshots WHERE channel_id = ? AND captured_at >= ? AND captured_at < ? ORDER BY captured_at ASC LIMIT 1000")
            .bind(id.to_string()).bind(from.to_rfc3339()).bind(to.to_rfc3339()).fetch_all(&self.pool).await.map_err(err)?;
        rows.iter()
            .map(|r| {
                Ok(ChannelMetricSnapshot {
                    id: Uuid::parse_str(&r.try_get::<String, _>("id").map_err(err)?)
                        .unwrap_or_default(),
                    channel_id: id,
                    provider: r
                        .try_get::<String, _>("provider")
                        .map_err(err)?
                        .parse::<Platform>()
                        .map_err(DomainError::Validation)?,
                    captured_at: dt(r.try_get("captured_at").map_err(err)?),
                    followers: r.try_get("followers").map_err(err)?,
                    total_views: r.try_get("total_views").map_err(err)?,
                    availability: availability(r.try_get("availability").map_err(err)?),
                    error_code: r.try_get("error_code").map_err(err)?,
                })
            })
            .collect()
    }

    async fn get_sync_state(
        &self,
        platform_account_id: Uuid,
    ) -> DomainResult<Option<AnalyticsSyncState>> {
        let row = sqlx::query("SELECT platform_account_id, provider, last_attempted_at, last_successful_at, next_allowed_at, last_error FROM analytics_sync_state WHERE platform_account_id = ?")
            .bind(platform_account_id.to_string()).fetch_optional(&self.pool).await.map_err(err)?;
        row.map(|r| {
            Ok(AnalyticsSyncState {
                platform_account_id,
                provider: r
                    .try_get::<String, _>("provider")
                    .map_err(err)?
                    .parse::<Platform>()
                    .map_err(DomainError::Validation)?,
                last_attempted_at: r
                    .try_get::<Option<String>, _>("last_attempted_at")
                    .map_err(err)?
                    .map(dt),
                last_successful_at: r
                    .try_get::<Option<String>, _>("last_successful_at")
                    .map_err(err)?
                    .map(dt),
                next_allowed_at: r
                    .try_get::<Option<String>, _>("next_allowed_at")
                    .map_err(err)?
                    .map(dt),
                last_error: r.try_get("last_error").map_err(err)?,
            })
        })
        .transpose()
    }

    async fn upsert_sync_state(&self, state: &AnalyticsSyncState) -> DomainResult<()> {
        sqlx::query("INSERT INTO analytics_sync_state (platform_account_id, provider, last_attempted_at, last_successful_at, next_allowed_at, last_error) VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT(platform_account_id) DO UPDATE SET provider = excluded.provider, last_attempted_at = excluded.last_attempted_at, last_successful_at = excluded.last_successful_at, next_allowed_at = excluded.next_allowed_at, last_error = excluded.last_error")
            .bind(state.platform_account_id.to_string()).bind(state.provider.as_str())
            .bind(state.last_attempted_at.map(|v| v.to_rfc3339())).bind(state.last_successful_at.map(|v| v.to_rfc3339()))
            .bind(state.next_allowed_at.map(|v| v.to_rfc3339())).bind(&state.last_error)
            .execute(&self.pool).await.map_err(err)?;
        Ok(())
    }
}
