use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::platform::Platform;
use crate::domain::ports::repositories::ScheduleSlotRepository;
use crate::domain::schedule_slot::ScheduleSlot;

use super::parse_dt;

pub struct SqliteScheduleSlotRepository {
    pool: SqlitePool,
}

impl SqliteScheduleSlotRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn map_repo_err(err: sqlx::Error) -> DomainError {
    DomainError::Repository(err.to_string())
}

const SELECT_COLUMNS: &str =
    "id, channel_id, platform, day_of_week, time_of_day, is_active, created_at, updated_at";

fn row_to_slot(row: &sqlx::sqlite::SqliteRow) -> Result<ScheduleSlot, DomainError> {
    let platform: Option<String> = row.try_get("platform").map_err(map_repo_err)?;
    Ok(ScheduleSlot {
        id: Uuid::parse_str(&row.try_get::<String, _>("id").map_err(map_repo_err)?)
            .unwrap_or_default(),
        channel_id: Uuid::parse_str(
            &row.try_get::<String, _>("channel_id")
                .map_err(map_repo_err)?,
        )
        .unwrap_or_default(),
        platform: platform
            .map(|p| p.parse::<Platform>().map_err(DomainError::Validation))
            .transpose()?,
        day_of_week: row.try_get("day_of_week").map_err(map_repo_err)?,
        time_of_day: row.try_get("time_of_day").map_err(map_repo_err)?,
        is_active: row.try_get::<i64, _>("is_active").map_err(map_repo_err)? != 0,
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
impl ScheduleSlotRepository for SqliteScheduleSlotRepository {
    async fn create(&self, slot: &ScheduleSlot) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO schedule_slots (id, channel_id, platform, day_of_week, time_of_day, is_active, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(slot.id.to_string())
        .bind(slot.channel_id.to_string())
        .bind(slot.platform.map(|p| p.as_str()))
        .bind(slot.day_of_week)
        .bind(&slot.time_of_day)
        .bind(slot.is_active)
        .bind(slot.created_at.to_rfc3339())
        .bind(slot.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|err| match err {
            sqlx::Error::Database(ref db_err) if db_err.is_unique_violation() => {
                DomainError::ScheduleConflict
            }
            other => map_repo_err(other),
        })?;
        Ok(())
    }

    async fn update(&self, slot: &ScheduleSlot) -> DomainResult<()> {
        sqlx::query(
            "UPDATE schedule_slots SET platform = ?, day_of_week = ?, time_of_day = ?, is_active = ?, updated_at = ? WHERE id = ?",
        )
        .bind(slot.platform.map(|p| p.as_str()))
        .bind(slot.day_of_week)
        .bind(&slot.time_of_day)
        .bind(slot.is_active)
        .bind(slot.updated_at.to_rfc3339())
        .bind(slot.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|err| match err {
            sqlx::Error::Database(ref db_err) if db_err.is_unique_violation() => {
                DomainError::ScheduleConflict
            }
            other => map_repo_err(other),
        })?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> DomainResult<()> {
        sqlx::query("DELETE FROM schedule_slots WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(map_repo_err)?;
        Ok(())
    }

    async fn get(&self, id: Uuid) -> DomainResult<Option<ScheduleSlot>> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM schedule_slots WHERE id = ?"
        ))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_repo_err)?;
        row.as_ref().map(row_to_slot).transpose()
    }

    async fn list_for_channel(&self, channel_id: Uuid) -> DomainResult<Vec<ScheduleSlot>> {
        let rows = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM schedule_slots WHERE channel_id = ? ORDER BY day_of_week ASC, time_of_day ASC"
        ))
        .bind(channel_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(map_repo_err)?;
        rows.iter().map(row_to_slot).collect()
    }

    async fn count_active_grouped_by_channel(
        &self,
        workspace_id: Uuid,
    ) -> DomainResult<Vec<(Uuid, i64)>> {
        let rows = sqlx::query(
            "SELECT s.channel_id AS channel_id, COUNT(*) AS c FROM schedule_slots s \
             JOIN channels c ON c.id = s.channel_id \
             WHERE c.workspace_id = ? AND s.is_active = 1 \
             GROUP BY s.channel_id",
        )
        .bind(workspace_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(map_repo_err)?;

        rows.iter()
            .map(|row| {
                let channel_id: String = row.try_get("channel_id").map_err(map_repo_err)?;
                let count: i64 = row.try_get("c").map_err(map_repo_err)?;
                Ok((Uuid::parse_str(&channel_id).unwrap_or_default(), count))
            })
            .collect()
    }
}
