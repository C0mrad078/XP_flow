use async_trait::async_trait;
use chrono::Utc;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::errors::{DomainError, DomainResult};
use crate::jobs::{Job, JobRepository, JobStatus, JobType};

use super::parse_dt;

pub struct SqliteJobRepository {
    pool: SqlitePool,
}

impl SqliteJobRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn map_repo_err(err: sqlx::Error) -> DomainError {
    DomainError::Repository(err.to_string())
}

fn row_to_job(row: &sqlx::sqlite::SqliteRow) -> Result<Job, DomainError> {
    let started_at: Option<String> = row.try_get("started_at").map_err(map_repo_err)?;
    let completed_at: Option<String> = row.try_get("completed_at").map_err(map_repo_err)?;

    Ok(Job {
        id: Uuid::parse_str(&row.try_get::<String, _>("id").map_err(map_repo_err)?)
            .unwrap_or_default(),
        job_type: row
            .try_get::<String, _>("job_type")
            .map_err(map_repo_err)?
            .parse::<JobType>()
            .map_err(DomainError::Validation)?,
        status: row
            .try_get::<String, _>("status")
            .map_err(map_repo_err)?
            .parse::<JobStatus>()
            .map_err(DomainError::Validation)?,
        payload_json: row.try_get("payload_json").map_err(map_repo_err)?,
        dedupe_key: row.try_get("dedupe_key").map_err(map_repo_err)?,
        created_at: parse_dt(
            &row.try_get::<String, _>("created_at")
                .map_err(map_repo_err)?,
        ),
        started_at: started_at.map(|s| parse_dt(&s)),
        completed_at: completed_at.map(|s| parse_dt(&s)),
        attempts: row.try_get("attempts").map_err(map_repo_err)?,
        last_error: row.try_get("last_error").map_err(map_repo_err)?,
    })
}

const SELECT_COLUMNS: &str =
    "id, job_type, status, payload_json, dedupe_key, created_at, started_at, completed_at, attempts, last_error";

#[async_trait]
impl JobRepository for SqliteJobRepository {
    async fn enqueue(&self, job: &Job) -> DomainResult<Job> {
        let result = sqlx::query(
            "INSERT INTO jobs (id, job_type, status, payload_json, dedupe_key, created_at, attempts) \
             VALUES (?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT (dedupe_key) WHERE dedupe_key IS NOT NULL AND status IN ('pending', 'running') DO NOTHING",
        )
        .bind(job.id.to_string())
        .bind(job.job_type.as_str())
        .bind(job.status.as_str())
        .bind(&job.payload_json)
        .bind(&job.dedupe_key)
        .bind(job.created_at.to_rfc3339())
        .bind(job.attempts)
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;

        if result.rows_affected() > 0 {
            return Ok(job.clone());
        }

        if let Some(dedupe_key) = &job.dedupe_key {
            let row = sqlx::query(&format!(
                "SELECT {SELECT_COLUMNS} FROM jobs WHERE dedupe_key = ? AND status IN ('pending', 'running') LIMIT 1"
            ))
            .bind(dedupe_key)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_repo_err)?;

            if let Some(row) = row {
                return row_to_job(&row);
            }
        }

        Ok(job.clone())
    }

    async fn mark_running(&self, id: Uuid) -> DomainResult<()> {
        sqlx::query("UPDATE jobs SET status = 'running', started_at = ?, attempts = attempts + 1 WHERE id = ?")
            .bind(Utc::now().to_rfc3339())
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(map_repo_err)?;
        Ok(())
    }

    async fn mark_succeeded(&self, id: Uuid) -> DomainResult<()> {
        sqlx::query("UPDATE jobs SET status = 'succeeded', completed_at = ? WHERE id = ?")
            .bind(Utc::now().to_rfc3339())
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(map_repo_err)?;
        Ok(())
    }

    async fn mark_failed(&self, id: Uuid, error: &str) -> DomainResult<()> {
        sqlx::query(
            "UPDATE jobs SET status = 'failed', completed_at = ?, last_error = ? WHERE id = ?",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(error)
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn fail_orphaned_running_jobs(&self, reason: &str) -> DomainResult<u64> {
        let result = sqlx::query("UPDATE jobs SET status = 'failed', completed_at = ?, last_error = ? WHERE status = 'running'")
            .bind(Utc::now().to_rfc3339())
            .bind(reason)
            .execute(&self.pool)
            .await
            .map_err(map_repo_err)?;
        Ok(result.rows_affected())
    }
}
