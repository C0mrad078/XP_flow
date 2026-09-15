use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::ports::repositories::ScheduleExceptionRepository;
use crate::domain::schedule_exception::{ScheduleException, ScheduleExceptionKind};

pub struct SqliteScheduleExceptionRepository {
    pool: SqlitePool,
}

impl SqliteScheduleExceptionRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn map_repo_err(err: sqlx::Error) -> DomainError {
    DomainError::Repository(err.to_string())
}

const SELECT_COLUMNS: &str = "id, channel_id, date, kind, reason, created_at";

fn row_to_exception(row: &sqlx::sqlite::SqliteRow) -> Result<ScheduleException, DomainError> {
    let date: String = row.try_get("date").map_err(map_repo_err)?;
    let created_at: String = row.try_get("created_at").map_err(map_repo_err)?;
    Ok(ScheduleException {
        id: Uuid::parse_str(&row.try_get::<String, _>("id").map_err(map_repo_err)?)
            .unwrap_or_default(),
        channel_id: Uuid::parse_str(
            &row.try_get::<String, _>("channel_id")
                .map_err(map_repo_err)?,
        )
        .unwrap_or_default(),
        date: NaiveDate::parse_from_str(&date, "%Y-%m-%d")
            .map_err(|e| DomainError::Repository(format!("invalid stored date {date:?}: {e}")))?,
        kind: row
            .try_get::<String, _>("kind")
            .map_err(map_repo_err)?
            .parse::<ScheduleExceptionKind>()
            .map_err(DomainError::Validation)?,
        reason: row.try_get("reason").map_err(map_repo_err)?,
        created_at: super::parse_dt(&created_at),
    })
}

#[async_trait]
impl ScheduleExceptionRepository for SqliteScheduleExceptionRepository {
    async fn create(&self, exception: &ScheduleException) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO schedule_exceptions (id, channel_id, date, kind, reason, created_at) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(exception.id.to_string())
        .bind(exception.channel_id.to_string())
        .bind(exception.date.format("%Y-%m-%d").to_string())
        .bind(exception.kind.as_str())
        .bind(&exception.reason)
        .bind(exception.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> DomainResult<()> {
        sqlx::query("DELETE FROM schedule_exceptions WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(map_repo_err)?;
        Ok(())
    }

    async fn list_for_channel_in_range(
        &self,
        channel_id: Uuid,
        from: NaiveDate,
        to: NaiveDate,
    ) -> DomainResult<Vec<ScheduleException>> {
        let rows = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM schedule_exceptions WHERE channel_id = ? AND date >= ? AND date <= ? ORDER BY date ASC"
        ))
        .bind(channel_id.to_string())
        .bind(from.format("%Y-%m-%d").to_string())
        .bind(to.format("%Y-%m-%d").to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(map_repo_err)?;
        rows.iter().map(row_to_exception).collect()
    }

    async fn exists_for_channel_date(
        &self,
        channel_id: Uuid,
        date: NaiveDate,
    ) -> DomainResult<bool> {
        let row = sqlx::query(
            "SELECT COUNT(*) AS c FROM schedule_exceptions WHERE channel_id = ? AND date = ?",
        )
        .bind(channel_id.to_string())
        .bind(date.format("%Y-%m-%d").to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(map_repo_err)?;
        let count: i64 = row.try_get("c").map_err(map_repo_err)?;
        Ok(count > 0)
    }
}
