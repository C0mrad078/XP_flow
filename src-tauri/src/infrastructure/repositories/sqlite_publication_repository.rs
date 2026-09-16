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
    let execution_key: Option<String> = row.try_get("execution_key").map_err(map_repo_err)?;
    let lease_expires_at: Option<String> = row.try_get("lease_expires_at").map_err(map_repo_err)?;
    let rendered_metadata_json: Option<String> = row
        .try_get("rendered_metadata_json")
        .map_err(map_repo_err)?;

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
        execution_key: execution_key.and_then(|s| Uuid::parse_str(&s).ok()),
        claim_token: row.try_get("claim_token").map_err(map_repo_err)?,
        lease_expires_at: lease_expires_at.map(|s| parse_dt(&s)),
        rendered_metadata: rendered_metadata_json.and_then(|json| serde_json::from_str(&json).ok()),
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
     description, hashtags_json, priority, locked, scheduled_at, published_at, remote_id, retry_count, last_error, \
     execution_key, claim_token, lease_expires_at, rendered_metadata_json, created_at, updated_at";

#[async_trait]
impl PublicationRepository for SqlitePublicationRepository {
    async fn create(&self, publication: &Publication) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO publications (id, workspace_id, video_id, channel_id, platform_account_id, platform, status, title, \
             description, hashtags_json, priority, locked, scheduled_at, published_at, remote_id, retry_count, last_error, \
             execution_key, claim_token, lease_expires_at, rendered_metadata_json, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
        .bind(publication.execution_key.map(|id| id.to_string()))
        .bind(&publication.claim_token)
        .bind(publication.lease_expires_at.map(|dt| dt.to_rfc3339()))
        .bind(
            publication
                .rendered_metadata
                .as_ref()
                .map(|m| serde_json::to_string(m).unwrap_or_default()),
        )
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
        // Deliberately excludes execution_key/claim_token/lease_expires_at/
        // rendered_metadata_json — see the trait doc comment. A caller
        // here (reschedule, cancel, a metadata edit) leaves those columns
        // exactly as they are in the database, regardless of whatever
        // stale value is sitting in `publication`'s in-memory copy of them.
        //
        // The `status NOT IN ('uploading', 'processing')` guard closes a
        // second, sharper version of the same hazard: without it, a
        // read-modify-write from an unrelated flow (its in-memory copy
        // loaded *before* the Publishing Engine claimed this row) would
        // silently revert `status` from `uploading` back to whatever it
        // was at read time — e.g. `scheduled` — which would make this
        // publication look due again to the next claim scan while the
        // original upload is still genuinely in flight, and hand it out
        // a second time. Once the engine owns a row, only
        // `update_execution_state` (guarded on the live claim token) may
        // touch it; a generic `update()` call against a currently-
        // claimed row affects zero rows and reports `Conflict` instead
        // of silently doing nothing.
        let result = sqlx::query(
            "UPDATE publications SET platform_account_id = ?, status = ?, title = ?, description = ?, hashtags_json = ?, \
             priority = ?, locked = ?, scheduled_at = ?, published_at = ?, remote_id = ?, retry_count = ?, last_error = ?, \
             updated_at = ? \
             WHERE id = ? AND status NOT IN ('uploading', 'processing')",
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
        if result.rows_affected() == 0 {
            return Err(DomainError::Conflict(
                "this publication is currently being published and can't be modified right now"
                    .to_string(),
            ));
        }
        Ok(())
    }

    async fn update_execution_state(
        &self,
        id: Uuid,
        claim_token: &str,
        update: &crate::domain::ports::repositories::ExecutionStateUpdate,
    ) -> DomainResult<bool> {
        let (new_claim_token, new_lease_expires_at): (Option<&str>, Option<String>) =
            if update.release_claim {
                (None, None)
            } else {
                (
                    Some(claim_token),
                    update.new_lease_expires_at.map(|dt| dt.to_rfc3339()),
                )
            };
        let result = sqlx::query(
            "UPDATE publications SET status = ?, remote_id = ?, retry_count = ?, last_error = ?, \
             rendered_metadata_json = COALESCE(?, rendered_metadata_json), claim_token = ?, lease_expires_at = ?, published_at = COALESCE(?, published_at), updated_at = ? \
             WHERE id = ? AND claim_token = ?",
        )
        .bind(update.status.as_str())
        .bind(&update.remote_id)
        .bind(update.retry_count)
        .bind(&update.last_error)
        .bind(&update.rendered_metadata_json)
        .bind(new_claim_token)
        .bind(new_lease_expires_at)
        .bind(update.published_at.map(|dt| dt.to_rfc3339()))
        .bind(Utc::now().to_rfc3339())
        .bind(id.to_string())
        .bind(claim_token)
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(result.rows_affected() == 1)
    }

    async fn try_finish_processing(
        &self,
        id: Uuid,
        status: PublicationStatus,
        remote_id: Option<String>,
        last_error: Option<String>,
        published_at: Option<DateTime<Utc>>,
    ) -> DomainResult<bool> {
        let result = sqlx::query(
            "UPDATE publications SET status = ?, remote_id = COALESCE(?, remote_id), last_error = ?, \
             published_at = COALESCE(?, published_at), claim_token = NULL, lease_expires_at = NULL, updated_at = ? \
             WHERE id = ? AND status = 'processing'",
        )
        .bind(status.as_str())
        .bind(remote_id)
        .bind(last_error)
        .bind(published_at.map(|dt| dt.to_rfc3339()))
        .bind(Utc::now().to_rfc3339())
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(result.rows_affected() == 1)
    }

    async fn bulk_update(&self, publications: &[Publication]) -> DomainResult<()> {
        if publications.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await.map_err(map_repo_err)?;

        for publication in publications {
            // Same claimed-row guard as `update()` — a bulk reschedule/
            // rebuild must never revert a publication the Publishing
            // Engine currently owns back to a pre-claim status.
            let result = sqlx::query(
                "UPDATE publications SET platform_account_id = ?, status = ?, title = ?, description = ?, hashtags_json = ?, \
                 priority = ?, locked = ?, scheduled_at = ?, published_at = ?, remote_id = ?, retry_count = ?, last_error = ?, updated_at = ? \
                 WHERE id = ? AND status NOT IN ('uploading', 'processing')",
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
                Ok(outcome) if outcome.rows_affected() == 1 => {}
                Ok(_) => {
                    let _ = tx.rollback().await;
                    return Err(DomainError::BulkScheduleFailed(format!(
                        "publication {} is currently being published and can't be rescheduled",
                        publication.id
                    )));
                }
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

    async fn try_claim_due(
        &self,
        id: Uuid,
        candidate_execution_key: Uuid,
        claim_token: &str,
        lease_expires_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> DomainResult<bool> {
        let result = sqlx::query(
            "UPDATE publications SET status = 'uploading', \
             execution_key = COALESCE(execution_key, ?), claim_token = ?, lease_expires_at = ?, updated_at = ? \
             WHERE id = ? AND status = 'scheduled' AND scheduled_at <= ? AND locked = 0",
        )
        .bind(candidate_execution_key.to_string())
        .bind(claim_token)
        .bind(lease_expires_at.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(id.to_string())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(result.rows_affected() == 1)
    }

    async fn list_with_expired_leases(
        &self,
        workspace_id: Uuid,
        now: DateTime<Utc>,
    ) -> DomainResult<Vec<Publication>> {
        let rows = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM publications \
             WHERE workspace_id = ? AND status IN ('uploading', 'processing') \
             AND lease_expires_at IS NOT NULL AND lease_expires_at <= ?"
        ))
        .bind(workspace_id.to_string())
        .bind(now.to_rfc3339())
        .fetch_all(&self.pool)
        .await
        .map_err(map_repo_err)?;
        rows.iter().map(row_to_publication).collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::video_status::VideoPriority;
    use crate::test_support::*;

    async fn seed_scheduled_publication(
        pool: &SqlitePool,
        workspace_id: Uuid,
        channel_id: Uuid,
        video_id: Uuid,
        scheduled_at: DateTime<Utc>,
    ) -> Publication {
        let mut publication = Publication::new(
            workspace_id,
            video_id,
            channel_id,
            Platform::YouTube,
            "Test publication",
            VideoPriority::Normal,
        );
        publication.status = PublicationStatus::Scheduled;
        publication.scheduled_at = Some(scheduled_at);
        let repo = SqlitePublicationRepository::new(pool.clone());
        repo.create(&publication).await.unwrap();
        publication
    }

    #[tokio::test]
    async fn try_claim_due_wins_for_a_due_unlocked_scheduled_publication() {
        let pool = temp_pool("pub-repo-claim").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Channel").await;
        let video_id = seed_video(&pool, workspace_id, source_id, Some(channel_id), "video").await;
        let publication = seed_scheduled_publication(
            &pool,
            workspace_id,
            channel_id,
            video_id,
            Utc::now() - chrono::Duration::minutes(1),
        )
        .await;
        let repo = SqlitePublicationRepository::new(pool.clone());

        let won = repo
            .try_claim_due(
                publication.id,
                Uuid::new_v4(),
                "claim-token-1",
                Utc::now() + chrono::Duration::minutes(30),
                Utc::now(),
            )
            .await
            .unwrap();
        assert!(won);

        let reloaded = repo.get(publication.id).await.unwrap().unwrap();
        assert_eq!(reloaded.status, PublicationStatus::Uploading);
        assert!(reloaded.execution_key.is_some());
        assert_eq!(reloaded.claim_token.as_deref(), Some("claim-token-1"));
    }

    #[tokio::test]
    async fn two_concurrent_claims_on_the_same_publication_only_one_wins() {
        let pool = temp_pool("pub-repo-claim-race").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Channel").await;
        let video_id = seed_video(&pool, workspace_id, source_id, Some(channel_id), "video").await;
        let publication = seed_scheduled_publication(
            &pool,
            workspace_id,
            channel_id,
            video_id,
            Utc::now() - chrono::Duration::minutes(1),
        )
        .await;
        let repo = SqlitePublicationRepository::new(pool.clone());

        let now = Utc::now();
        let lease = now + chrono::Duration::minutes(30);
        let (a, b) = tokio::join!(
            repo.try_claim_due(publication.id, Uuid::new_v4(), "claim-a", lease, now),
            repo.try_claim_due(publication.id, Uuid::new_v4(), "claim-b", lease, now)
        );
        let wins = [a.unwrap(), b.unwrap()].into_iter().filter(|w| *w).count();
        assert_eq!(wins, 1, "exactly one of two concurrent claims should win");
    }

    #[tokio::test]
    async fn a_locked_publication_is_never_claimed() {
        let pool = temp_pool("pub-repo-claim-locked").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Channel").await;
        let video_id = seed_video(&pool, workspace_id, source_id, Some(channel_id), "video").await;
        let mut publication = seed_scheduled_publication(
            &pool,
            workspace_id,
            channel_id,
            video_id,
            Utc::now() - chrono::Duration::minutes(1),
        )
        .await;
        publication.locked = true;
        let repo = SqlitePublicationRepository::new(pool.clone());
        repo.update(&publication).await.unwrap();

        let won = repo
            .try_claim_due(
                publication.id,
                Uuid::new_v4(),
                "claim-token",
                Utc::now() + chrono::Duration::minutes(30),
                Utc::now(),
            )
            .await
            .unwrap();
        assert!(!won);
    }

    #[tokio::test]
    async fn retrying_the_same_publication_reuses_its_execution_key() {
        let pool = temp_pool("pub-repo-claim-reuse-key").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Channel").await;
        let video_id = seed_video(&pool, workspace_id, source_id, Some(channel_id), "video").await;
        let publication = seed_scheduled_publication(
            &pool,
            workspace_id,
            channel_id,
            video_id,
            Utc::now() - chrono::Duration::minutes(1),
        )
        .await;
        let repo = SqlitePublicationRepository::new(pool.clone());
        let lease = Utc::now() + chrono::Duration::minutes(30);

        repo.try_claim_due(publication.id, Uuid::new_v4(), "claim-1", lease, Utc::now())
            .await
            .unwrap();
        let first_key = repo
            .get(publication.id)
            .await
            .unwrap()
            .unwrap()
            .execution_key;

        // Simulate the failure/retry cycle through the real state machine:
        // Uploading -> Failed (releasing the claim) -> RetryWait -> Queued
        // -> Scheduled -> claimed again.
        repo.update_execution_state(
            publication.id,
            "claim-1",
            &crate::domain::ports::repositories::ExecutionStateUpdate {
                status: PublicationStatus::Failed,
                remote_id: None,
                retry_count: 1,
                last_error: Some("network error".to_string()),
                rendered_metadata_json: None,
                release_claim: true,
                new_lease_expires_at: None,
                published_at: None,
            },
        )
        .await
        .unwrap();

        let mut reloaded = repo.get(publication.id).await.unwrap().unwrap();
        reloaded.transition(PublicationStatus::RetryWait).unwrap();
        reloaded.transition(PublicationStatus::Queued).unwrap();
        reloaded.transition(PublicationStatus::Scheduled).unwrap();
        repo.update(&reloaded).await.unwrap();

        repo.try_claim_due(publication.id, Uuid::new_v4(), "claim-2", lease, Utc::now())
            .await
            .unwrap();
        let second_key = repo
            .get(publication.id)
            .await
            .unwrap()
            .unwrap()
            .execution_key;

        assert_eq!(first_key, second_key);
    }

    #[tokio::test]
    async fn list_with_expired_leases_only_returns_abandoned_claims() {
        let pool = temp_pool("pub-repo-expired-leases").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Channel").await;
        let video_id = seed_video(&pool, workspace_id, source_id, Some(channel_id), "video").await;
        let repo = SqlitePublicationRepository::new(pool.clone());

        let expired =
            seed_scheduled_publication(&pool, workspace_id, channel_id, video_id, Utc::now()).await;
        repo.try_claim_due(
            expired.id,
            Uuid::new_v4(),
            "t1",
            Utc::now() - chrono::Duration::minutes(1),
            Utc::now(),
        )
        .await
        .unwrap();

        let fresh_video =
            seed_video(&pool, workspace_id, source_id, Some(channel_id), "video2").await;
        let fresh =
            seed_scheduled_publication(&pool, workspace_id, channel_id, fresh_video, Utc::now())
                .await;
        repo.try_claim_due(
            fresh.id,
            Uuid::new_v4(),
            "t2",
            Utc::now() + chrono::Duration::minutes(30),
            Utc::now(),
        )
        .await
        .unwrap();

        let expired_leases = repo
            .list_with_expired_leases(workspace_id, Utc::now())
            .await
            .unwrap();
        assert_eq!(expired_leases.len(), 1);
        assert_eq!(expired_leases[0].id, expired.id);
    }

    #[tokio::test]
    async fn update_execution_state_succeeds_only_for_the_current_claim_token() {
        let pool = temp_pool("pub-repo-exec-state").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Channel").await;
        let video_id = seed_video(&pool, workspace_id, source_id, Some(channel_id), "video").await;
        let publication =
            seed_scheduled_publication(&pool, workspace_id, channel_id, video_id, Utc::now()).await;
        let repo = SqlitePublicationRepository::new(pool.clone());
        repo.try_claim_due(
            publication.id,
            Uuid::new_v4(),
            "real-claim-token",
            Utc::now() + chrono::Duration::minutes(30),
            Utc::now(),
        )
        .await
        .unwrap();

        let update = crate::domain::ports::repositories::ExecutionStateUpdate {
            status: PublicationStatus::Processing,
            remote_id: Some("remote-1".to_string()),
            retry_count: 0,
            last_error: None,
            rendered_metadata_json: None,
            release_claim: false,
            new_lease_expires_at: Some(Utc::now() + chrono::Duration::minutes(45)),
            published_at: None,
        };

        // A stale/wrong token must be rejected, not silently applied.
        let stale_result = repo
            .update_execution_state(publication.id, "wrong-token", &update)
            .await
            .unwrap();
        assert!(!stale_result);
        let unchanged = repo.get(publication.id).await.unwrap().unwrap();
        assert_eq!(unchanged.status, PublicationStatus::Uploading);

        // The real token succeeds.
        let real_result = repo
            .update_execution_state(publication.id, "real-claim-token", &update)
            .await
            .unwrap();
        assert!(real_result);
        let updated = repo.get(publication.id).await.unwrap().unwrap();
        assert_eq!(updated.status, PublicationStatus::Processing);
        assert_eq!(updated.remote_id.as_deref(), Some("remote-1"));
        assert_eq!(updated.claim_token.as_deref(), Some("real-claim-token"));
    }

    #[tokio::test]
    async fn a_generic_update_against_a_currently_claimed_row_fails_loudly_instead_of_reverting_it()
    {
        let pool = temp_pool("pub-repo-generic-update-safe").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Channel").await;
        let video_id = seed_video(&pool, workspace_id, source_id, Some(channel_id), "video").await;
        let publication =
            seed_scheduled_publication(&pool, workspace_id, channel_id, video_id, Utc::now()).await;
        let repo = SqlitePublicationRepository::new(pool.clone());
        repo.try_claim_due(
            publication.id,
            Uuid::new_v4(),
            "active-claim",
            Utc::now() + chrono::Duration::minutes(30),
            Utc::now(),
        )
        .await
        .unwrap();

        // Simulates an unrelated flow (e.g. a title edit) that loaded the
        // publication *before* the claim above, still holding a stale
        // (Scheduled) status and empty claim fields in memory.
        let mut stale_in_memory_copy = publication.clone();
        stale_in_memory_copy.title = "Edited title".to_string();
        let result = repo.update(&stale_in_memory_copy).await;

        assert!(
            matches!(result, Err(DomainError::Conflict(_))),
            "an update against a claimed row must fail, not silently revert the claim"
        );

        let reloaded = repo.get(publication.id).await.unwrap().unwrap();
        assert_eq!(
            reloaded.title, "Test publication",
            "the edit must not have applied"
        );
        assert_eq!(
            reloaded.status,
            PublicationStatus::Uploading,
            "the concurrently-made claim's status must survive the rejected update"
        );
        assert_eq!(reloaded.claim_token.as_deref(), Some("active-claim"));
    }

    #[tokio::test]
    async fn try_finish_processing_only_applies_to_a_processing_row() {
        let pool = temp_pool("pub-repo-finish-processing").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Channel").await;
        let video_id = seed_video(&pool, workspace_id, source_id, Some(channel_id), "video").await;
        let publication =
            seed_scheduled_publication(&pool, workspace_id, channel_id, video_id, Utc::now()).await;
        let repo = SqlitePublicationRepository::new(pool.clone());

        // Not Processing yet — must be rejected.
        let rejected = repo
            .try_finish_processing(
                publication.id,
                PublicationStatus::Published,
                None,
                None,
                Some(Utc::now()),
            )
            .await
            .unwrap();
        assert!(!rejected);

        repo.try_claim_due(
            publication.id,
            Uuid::new_v4(),
            "claim-1",
            Utc::now() + chrono::Duration::minutes(30),
            Utc::now(),
        )
        .await
        .unwrap();
        repo.update_execution_state(
            publication.id,
            "claim-1",
            &crate::domain::ports::repositories::ExecutionStateUpdate {
                status: PublicationStatus::Processing,
                remote_id: Some("remote-processing-1".to_string()),
                retry_count: 0,
                last_error: None,
                rendered_metadata_json: None,
                release_claim: true,
                new_lease_expires_at: None,
                published_at: None,
            },
        )
        .await
        .unwrap();

        let now = Utc::now();
        let first = repo
            .try_finish_processing(
                publication.id,
                PublicationStatus::Published,
                None,
                None,
                Some(now),
            )
            .await
            .unwrap();
        assert!(first);
        let reloaded = repo.get(publication.id).await.unwrap().unwrap();
        assert_eq!(reloaded.status, PublicationStatus::Published);
        assert_eq!(reloaded.remote_id.as_deref(), Some("remote-processing-1"));
        assert!(reloaded.published_at.is_some());

        // A second, redundant poll result must be a harmless no-op.
        let second = repo
            .try_finish_processing(publication.id, PublicationStatus::Failed, None, None, None)
            .await
            .unwrap();
        assert!(!second);
        let unchanged = repo.get(publication.id).await.unwrap().unwrap();
        assert_eq!(unchanged.status, PublicationStatus::Published);
    }
}
