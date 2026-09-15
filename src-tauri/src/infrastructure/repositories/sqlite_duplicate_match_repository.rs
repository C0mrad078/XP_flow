use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::duplicate_match::{DuplicateMatch, MatchType};
use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::ports::repositories::DuplicateMatchRepository;

use super::parse_dt;

pub struct SqliteDuplicateMatchRepository {
    pool: SqlitePool,
}

impl SqliteDuplicateMatchRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn map_repo_err(err: sqlx::Error) -> DomainError {
    DomainError::Repository(err.to_string())
}

#[async_trait]
impl DuplicateMatchRepository for SqliteDuplicateMatchRepository {
    async fn create(&self, duplicate: &DuplicateMatch) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO duplicate_matches (id, video_id, matched_video_id, similarity, match_type, created_at) \
             VALUES (?, ?, ?, ?, ?, ?) \
             ON CONFLICT(video_id, matched_video_id) DO UPDATE SET similarity = excluded.similarity",
        )
        .bind(duplicate.id.to_string())
        .bind(duplicate.video_id.to_string())
        .bind(duplicate.matched_video_id.to_string())
        .bind(duplicate.similarity)
        .bind(duplicate.match_type.as_str())
        .bind(duplicate.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn list_for_video(&self, video_id: Uuid) -> DomainResult<Vec<DuplicateMatch>> {
        let rows = sqlx::query(
            "SELECT id, video_id, matched_video_id, similarity, match_type, created_at \
             FROM duplicate_matches WHERE video_id = ? ORDER BY similarity DESC",
        )
        .bind(video_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(map_repo_err)?;

        rows.iter()
            .map(|row| {
                Ok(DuplicateMatch {
                    id: Uuid::parse_str(&row.try_get::<String, _>("id").map_err(map_repo_err)?)
                        .unwrap_or_default(),
                    video_id: Uuid::parse_str(
                        &row.try_get::<String, _>("video_id").map_err(map_repo_err)?,
                    )
                    .unwrap_or_default(),
                    matched_video_id: Uuid::parse_str(
                        &row.try_get::<String, _>("matched_video_id")
                            .map_err(map_repo_err)?,
                    )
                    .unwrap_or_default(),
                    similarity: row.try_get("similarity").map_err(map_repo_err)?,
                    match_type: row
                        .try_get::<String, _>("match_type")
                        .map_err(map_repo_err)?
                        .parse::<MatchType>()
                        .map_err(DomainError::Validation)?,
                    created_at: parse_dt(
                        &row.try_get::<String, _>("created_at")
                            .map_err(map_repo_err)?,
                    ),
                })
            })
            .collect()
    }
}
