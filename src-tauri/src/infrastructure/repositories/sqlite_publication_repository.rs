use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool};
use uuid::Uuid;

use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::platform::Platform;
use crate::domain::ports::repositories::PublicationRepository;
use crate::domain::publication::{Publication, PublicationStatus};
use crate::domain::publication_query::{PublicationListQuery, PublicationPage, QueueSort};
use crate::domain::video_status::VideoPriority;

use super::parse_dt;

pub struct SqlitePublicationRepository {
    pool: SqlitePool,
}

impl SqlitePublicationRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn map_repo_err(err: sqlx::Error) -> DomainError {
    DomainError::Repository(err.to_string())
}

fn row_to_publication(row: &sqlx::sqlite::SqliteRow) -> Result<Publication, DomainError> {
    let hashtags_json: String = row.try_get("hashtags_json").map_err(map_repo_err)?;
    let platform_account_id: Option<String> =
        row.try_get("platform_account_id").map_err(map_repo_err)?;
    let scheduled_at: Option<String> = row.try_get("scheduled_at").map_err(map_repo_err)?;
    let published_at: Option<String> = row.try_get("published_at").map_err(map_repo_err)?;

    Ok(Publication {
        id: Uuid::parse_str(&row.try_get::<String, _>("id").map_err(map_repo_err)?)
            .unwrap_or_default(),
        workspace_id: Uuid::parse_str(
            &row.try_get::<String, _>("workspace_id")
                .map_err(map_repo_err)?,
        )
        .unwrap_or_default(),
        video_id: Uuid::parse_str(&row.try_get::<String, _>("video_id").map_err(map_repo_err)?)
            .unwrap_or_default(),
        channel_id: Uuid::parse_str(
            &row.try_get::<String, _>("channel_id")
                .map_err(map_repo_err)?,
        )
        .unwrap_or_default(),
        platform_account_id: platform_account_id.and_then(|s| Uuid::parse_str(&s).ok()),
        platform: row
            .try_get::<String, _>("platform")
            .map_err(map_repo_err)?
            .parse::<Platform>()
            .map_err(DomainError::Validation)?,
        status: row
            .try_get::<String, _>("status")
            .map_err(map_repo_err)?
            .parse::<PublicationStatus>()
            .map_err(DomainError::Validation)?,
        title: row.try_get("title").map_err(map_repo_err)?,
        description: row.try_get("description").map_err(map_repo_err)?,
        hashtags: serde_json::from_str(&hashtags_json).unwrap_or_default(),
        priority: row
            .try_get::<String, _>("priority")
            .map_err(map_repo_err)?
            .parse::<VideoPriority>()
            .map_err(DomainError::Validation)?,
        locked: row.try_get::<i64, _>("locked").map_err(map_repo_err)? != 0,
        scheduled_at: scheduled_at.map(|s| parse_dt(&s)),
        published_at: published_at.map(|s| parse_dt(&s)),
        remote_id: row.try_get("remote_id").map_err(map_repo_err)?,
        retry_count: row.try_get("retry_count").map_err(map_repo_err)?,
        last_error: row.try_get("last_error").map_err(map_repo_err)?,
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

const SELECT_COLUMNS: &str = "id, workspace_id, video_id, channel_id, platform_account_id, platform, status, title, \
     description, hashtags_json, priority, locked, scheduled_at, published_at, remote_id, retry_count, last_error, created_at, updated_at";

#[async_trait]
impl PublicationRepository for SqlitePublicationRepository {
    async fn create(&self, publication: &Publication) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO publications (id, workspace_id, video_id, channel_id, platform_account_id, platform, status, title, \
             description, hashtags_json, priority, locked, scheduled_at, published_at, remote_id, retry_count, last_error, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(publication.id.to_string())
        .bind(publication.workspace_id.to_string())
        .bind(publication.video_id.to_string())
        .bind(publication.channel_id.to_string())
        .bind(publication.platform_account_id.map(|id| id.to_string()))
        .bind(publication.platform.as_str())
        .bind(publication.status.as_str())
        .bind(&publication.title)
        .bind(&publication.description)
        .bind(serde_json::to_string(&publication.hashtags).unwrap_or_else(|_| "[]".to_string()))
        .bind(publication.priority.as_str())
        .bind(publication.locked)
        .bind(publication.scheduled_at.map(|dt| dt.to_rfc3339()))
        .bind(publication.published_at.map(|dt| dt.to_rfc3339()))
        .bind(&publication.remote_id)
        .bind(publication.retry_count)
        .bind(&publication.last_error)
        .bind(publication.created_at.to_rfc3339())
        .bind(publication.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|err| match err {
            sqlx::Error::Database(ref db_err) if db_err.is_unique_violation() => {
                DomainError::PublicationAlreadyExists
            }
            other => map_repo_err(other),
        })?;
        Ok(())
    }

    async fn update(&self, publication: &Publication) -> DomainResult<()> {
        sqlx::query(
            "UPDATE publications SET platform_account_id = ?, status = ?, title = ?, description = ?, hashtags_json = ?, \
             priority = ?, locked = ?, scheduled_at = ?, published_at = ?, remote_id = ?, retry_count = ?, last_error = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(publication.platform_account_id.map(|id| id.to_string()))
        .bind(publication.status.as_str())
        .bind(&publication.title)
        .bind(&publication.description)
        .bind(serde_json::to_string(&publication.hashtags).unwrap_or_else(|_| "[]".to_string()))
        .bind(publication.priority.as_str())
        .bind(publication.locked)
        .bind(publication.scheduled_at.map(|dt| dt.to_rfc3339()))
        .bind(publication.published_at.map(|dt| dt.to_rfc3339()))
        .bind(&publication.remote_id)
        .bind(publication.retry_count)
        .bind(&publication.last_error)
        .bind(publication.updated_at.to_rfc3339())
        .bind(publication.id.to_string())
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

    async fn bulk_update(&self, publications: &[Publication]) -> DomainResult<()> {
        if publications.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await.map_err(map_repo_err)?;

        for publication in publications {
            let result = sqlx::query(
                "UPDATE publications SET platform_account_id = ?, status = ?, title = ?, description = ?, hashtags_json = ?, \
                 priority = ?, locked = ?, scheduled_at = ?, published_at = ?, remote_id = ?, retry_count = ?, last_error = ?, updated_at = ? \
                 WHERE id = ?",
            )
            .bind(publication.platform_account_id.map(|id| id.to_string()))
            .bind(publication.status.as_str())
            .bind(&publication.title)
            .bind(&publication.description)
            .bind(serde_json::to_string(&publication.hashtags).unwrap_or_else(|_| "[]".to_string()))
            .bind(publication.priority.as_str())
            .bind(publication.locked)
            .bind(publication.scheduled_at.map(|dt| dt.to_rfc3339()))
            .bind(publication.published_at.map(|dt| dt.to_rfc3339()))
            .bind(&publication.remote_id)
            .bind(publication.retry_count)
            .bind(&publication.last_error)
            .bind(publication.updated_at.to_rfc3339())
            .bind(publication.id.to_string())
            .execute(&mut *tx)
            .await;

            match result {
                Ok(_) => {}
                Err(sqlx::Error::Database(ref db_err)) if db_err.is_unique_violation() => {
                    let _ = tx.rollback().await;
                    return Err(DomainError::BulkScheduleFailed(format!(
                        "publication {} conflicts with an existing schedule slot",
                        publication.id
                    )));
                }
                Err(other) => {
                    let _ = tx.rollback().await;
                    return Err(DomainError::BulkScheduleFailed(other.to_string()));
                }
            }
        }

        tx.commit()
            .await
            .map_err(|e| DomainError::BulkScheduleFailed(format!("failed to commit batch: {e}")))?;
        Ok(())
    }

    async fn get(&self, id: Uuid) -> DomainResult<Option<Publication>> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM publications WHERE id = ?"
        ))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_repo_err)?;
        row.as_ref().map(row_to_publication).transpose()
    }

    async fn list_for_video(&self, video_id: Uuid) -> DomainResult<Vec<Publication>> {
        let rows = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM publications WHERE video_id = ? ORDER BY created_at ASC"
        ))
        .bind(video_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(map_repo_err)?;

        rows.iter().map(row_to_publication).collect()
    }

    async fn find_active_for_video_channel_platform(
        &self,
        video_id: Uuid,
        channel_id: Uuid,
        platform: Platform,
    ) -> DomainResult<Option<Publication>> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM publications \
             WHERE video_id = ? AND channel_id = ? AND platform = ? \
             AND status NOT IN ('cancelled', 'archived', 'duplicate') \
             LIMIT 1"
        ))
        .bind(video_id.to_string())
        .bind(channel_id.to_string())
        .bind(platform.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_repo_err)?;
        row.as_ref().map(row_to_publication).transpose()
    }

    async fn list_paginated(&self, query: &PublicationListQuery) -> DomainResult<PublicationPage> {
        let mut count_builder: QueryBuilder<Sqlite> =
            QueryBuilder::new("SELECT COUNT(*) FROM publications WHERE 1 = 1");
        apply_filters(&mut count_builder, query);
        let total: i64 = count_builder
            .build_query_scalar()
            .fetch_one(&self.pool)
            .await
            .map_err(map_repo_err)?;

        let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(format!(
            "SELECT {SELECT_COLUMNS} FROM publications WHERE 1 = 1"
        ));
        apply_filters(&mut builder, query);

        let order_by = match query.sort {
            // Unscheduled items sort by their manual queue position (via a
            // join would be needed for a real "position" column ordering,
            // but for a mixed list a stable, sensible default is: scheduled
            // items by scheduled_at, unscheduled by priority then recency).
            QueueSort::QueueOrder => {
                " ORDER BY (scheduled_at IS NULL) DESC, scheduled_at ASC, \
                  CASE priority WHEN 'urgent' THEN 0 WHEN 'high' THEN 1 WHEN 'normal' THEN 2 ELSE 3 END ASC, \
                  created_at ASC"
            }
            QueueSort::PriorityDesc => {
                " ORDER BY CASE priority WHEN 'urgent' THEN 0 WHEN 'high' THEN 1 WHEN 'normal' THEN 2 ELSE 3 END ASC, created_at ASC"
            }
            QueueSort::NewestFirst => " ORDER BY created_at DESC",
            QueueSort::OldestFirst => " ORDER BY created_at ASC",
        };
        builder.push(order_by);
        builder.push(" LIMIT ").push_bind(query.page_size);
        builder
            .push(" OFFSET ")
            .push_bind(query.page * query.page_size);

        let rows = builder
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(map_repo_err)?;
        let items = rows
            .iter()
            .map(row_to_publication)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(PublicationPage {
            items,
            total,
            page: query.page,
            page_size: query.page_size,
        })
    }

    async fn list_scheduled_in_range(
        &self,
        workspace_id: Uuid,
        channel_id: Option<Uuid>,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> DomainResult<Vec<Publication>> {
        let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(format!(
            "SELECT {SELECT_COLUMNS} FROM publications \
             WHERE workspace_id = "
        ));
        builder.push_bind(workspace_id.to_string());
        builder.push(" AND status = 'scheduled' AND scheduled_at >= ");
        builder.push_bind(from.to_rfc3339());
        builder.push(" AND scheduled_at < ");
        builder.push_bind(to.to_rfc3339());
        if let Some(channel_id) = channel_id {
            builder.push(" AND channel_id = ");
            builder.push_bind(channel_id.to_string());
        }
        builder.push(" ORDER BY scheduled_at ASC");

        let rows = builder
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(map_repo_err)?;
        rows.iter().map(row_to_publication).collect()
    }

    async fn list_due(
        &self,
        workspace_id: Uuid,
        now: DateTime<Utc>,
    ) -> DomainResult<Vec<Publication>> {
        let rows = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM publications \
             WHERE workspace_id = ? AND status = 'scheduled' AND scheduled_at <= ? \
             ORDER BY scheduled_at ASC"
        ))
        .bind(workspace_id.to_string())
        .bind(now.to_rfc3339())
        .fetch_all(&self.pool)
        .await
        .map_err(map_repo_err)?;
        rows.iter().map(row_to_publication).collect()
    }

    async fn list_unscheduled_for_channel(
        &self,
        channel_id: Uuid,
    ) -> DomainResult<Vec<Publication>> {
        let rows = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM publications \
             WHERE channel_id = ? AND status = 'queued' AND locked = 0 \
             ORDER BY CASE priority WHEN 'urgent' THEN 0 WHEN 'high' THEN 1 WHEN 'normal' THEN 2 ELSE 3 END ASC, created_at ASC"
        ))
        .bind(channel_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(map_repo_err)?;
        rows.iter().map(row_to_publication).collect()
    }

    async fn count_active_grouped_by_channel(
        &self,
        workspace_id: Uuid,
    ) -> DomainResult<Vec<(Uuid, i64)>> {
        let rows = sqlx::query(
            "SELECT channel_id, COUNT(*) AS c FROM publications \
             WHERE workspace_id = ? AND status IN ('queued', 'scheduled') \
             GROUP BY channel_id",
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

fn apply_filters(builder: &mut QueryBuilder<Sqlite>, query: &PublicationListQuery) {
    builder
        .push(" AND workspace_id = ")
        .push_bind(query.workspace_id.to_string());

    if let Some(channel_id) = query.channel_id {
        builder
            .push(" AND channel_id = ")
            .push_bind(channel_id.to_string());
    }
    if let Some(platform) = query.platform {
        builder
            .push(" AND platform = ")
            .push_bind(platform.as_str());
    }
    if let Some(priority) = query.priority {
        builder
            .push(" AND priority = ")
            .push_bind(priority.as_str());
    }
    if let Some(statuses) = &query.statuses {
        if !statuses.is_empty() {
            builder.push(" AND status IN (");
            let mut separated = builder.separated(", ");
            for status in statuses {
                separated.push_bind(status.as_str());
            }
            separated.push_unseparated(")");
        }
    }
    if let Some(search) = &query.search {
        if !search.trim().is_empty() {
            let pattern = format!("%{}%", search.trim());
            builder.push(" AND title LIKE ").push_bind(pattern);
        }
    }
}
