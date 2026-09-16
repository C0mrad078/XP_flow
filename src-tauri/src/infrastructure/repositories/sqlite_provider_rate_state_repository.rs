use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::ports::repositories::ProviderRateStateRepository;

use super::parse_dt;

pub struct SqliteProviderRateStateRepository {
    pool: SqlitePool,
}

impl SqliteProviderRateStateRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn map_repo_err(err: sqlx::Error) -> DomainError {
    DomainError::Repository(err.to_string())
}

#[async_trait]
impl ProviderRateStateRepository for SqliteProviderRateStateRepository {
    async fn record_rate_limit(
        &self,
        platform_account_id: Uuid,
        operation: &str,
        retry_after: DateTime<Utc>,
    ) -> DomainResult<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO provider_rate_state (id, platform_account_id, operation, retry_after, last_rate_limited_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?) \
             ON CONFLICT(platform_account_id, operation) DO UPDATE SET \
             retry_after = excluded.retry_after, last_rate_limited_at = excluded.last_rate_limited_at, updated_at = excluded.updated_at",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(platform_account_id.to_string())
        .bind(operation)
        .bind(retry_after.to_rfc3339())
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn get_retry_after(
        &self,
        platform_account_id: Uuid,
        operation: &str,
    ) -> DomainResult<Option<DateTime<Utc>>> {
        let row = sqlx::query("SELECT retry_after FROM provider_rate_state WHERE platform_account_id = ? AND operation = ?")
            .bind(platform_account_id.to_string())
            .bind(operation)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_repo_err)?;
        match row {
            None => Ok(None),
            Some(row) => {
                let retry_after: Option<String> =
                    row.try_get("retry_after").map_err(map_repo_err)?;
                Ok(retry_after.map(|s| parse_dt(&s)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{
        seed_channel, seed_platform_account, seed_workspace_and_source, temp_pool,
    };

    #[tokio::test]
    async fn recording_then_reading_round_trips() {
        let pool = temp_pool("rate-state-repo").await;
        let (workspace_id, _source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Channel").await;
        let account_id = seed_platform_account(&pool, workspace_id, channel_id).await;
        let repo = SqliteProviderRateStateRepository::new(pool);

        assert!(repo
            .get_retry_after(account_id, "publish")
            .await
            .unwrap()
            .is_none());

        let retry_after = Utc::now() + chrono::Duration::minutes(15);
        repo.record_rate_limit(account_id, "publish", retry_after)
            .await
            .unwrap();

        let stored = repo
            .get_retry_after(account_id, "publish")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.timestamp(), retry_after.timestamp());
    }

    #[tokio::test]
    async fn recording_again_overwrites_the_previous_window() {
        let pool = temp_pool("rate-state-repo-overwrite").await;
        let (workspace_id, _source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Channel").await;
        let account_id = seed_platform_account(&pool, workspace_id, channel_id).await;
        let repo = SqliteProviderRateStateRepository::new(pool);

        repo.record_rate_limit(
            account_id,
            "publish",
            Utc::now() + chrono::Duration::minutes(5),
        )
        .await
        .unwrap();
        let later = Utc::now() + chrono::Duration::minutes(30);
        repo.record_rate_limit(account_id, "publish", later)
            .await
            .unwrap();

        let stored = repo
            .get_retry_after(account_id, "publish")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.timestamp(), later.timestamp());
    }
}
