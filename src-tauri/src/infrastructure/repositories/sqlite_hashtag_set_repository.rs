use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::platform::Platform;
use crate::domain::ports::repositories::HashtagSetRepository;
use crate::domain::publishing::HashtagSet;

use super::parse_dt;

pub struct SqliteHashtagSetRepository {
    pool: SqlitePool,
}

impl SqliteHashtagSetRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn map_repo_err(err: sqlx::Error) -> DomainError {
    DomainError::Repository(err.to_string())
}

const SELECT_COLUMNS: &str =
    "id, workspace_id, channel_id, platform, name, hashtags_json, created_at, updated_at";

fn row_to_set(row: &sqlx::sqlite::SqliteRow) -> Result<HashtagSet, DomainError> {
    let channel_id: Option<String> = row.try_get("channel_id").map_err(map_repo_err)?;
    let platform: Option<String> = row.try_get("platform").map_err(map_repo_err)?;
    let hashtags_json: String = row.try_get("hashtags_json").map_err(map_repo_err)?;

    Ok(HashtagSet {
        id: Uuid::parse_str(&row.try_get::<String, _>("id").map_err(map_repo_err)?)
            .unwrap_or_default(),
        workspace_id: Uuid::parse_str(
            &row.try_get::<String, _>("workspace_id")
                .map_err(map_repo_err)?,
        )
        .unwrap_or_default(),
        channel_id: channel_id.and_then(|s| Uuid::parse_str(&s).ok()),
        platform: platform
            .map(|p| p.parse::<Platform>().map_err(DomainError::Validation))
            .transpose()?,
        name: row.try_get("name").map_err(map_repo_err)?,
        hashtags: serde_json::from_str(&hashtags_json).unwrap_or_default(),
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
impl HashtagSetRepository for SqliteHashtagSetRepository {
    async fn create(&self, set: &HashtagSet) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO hashtag_sets (id, workspace_id, channel_id, platform, name, hashtags_json, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(set.id.to_string())
        .bind(set.workspace_id.to_string())
        .bind(set.channel_id.map(|id| id.to_string()))
        .bind(set.platform.map(|p| p.as_str()))
        .bind(&set.name)
        .bind(serde_json::to_string(&set.hashtags).unwrap_or_else(|_| "[]".to_string()))
        .bind(set.created_at.to_rfc3339())
        .bind(set.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn update(&self, set: &HashtagSet) -> DomainResult<()> {
        sqlx::query(
            "UPDATE hashtag_sets SET channel_id = ?, platform = ?, name = ?, hashtags_json = ?, updated_at = ? WHERE id = ?",
        )
        .bind(set.channel_id.map(|id| id.to_string()))
        .bind(set.platform.map(|p| p.as_str()))
        .bind(&set.name)
        .bind(serde_json::to_string(&set.hashtags).unwrap_or_else(|_| "[]".to_string()))
        .bind(set.updated_at.to_rfc3339())
        .bind(set.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> DomainResult<()> {
        sqlx::query("DELETE FROM hashtag_sets WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(map_repo_err)?;
        Ok(())
    }

    async fn get(&self, id: Uuid) -> DomainResult<Option<HashtagSet>> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM hashtag_sets WHERE id = ?"
        ))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_repo_err)?;
        row.as_ref().map(row_to_set).transpose()
    }

    async fn list_for_workspace(&self, workspace_id: Uuid) -> DomainResult<Vec<HashtagSet>> {
        let rows = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM hashtag_sets WHERE workspace_id = ? ORDER BY created_at ASC"
        ))
        .bind(workspace_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(map_repo_err)?;
        rows.iter().map(row_to_set).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{seed_workspace_and_source, temp_pool};

    #[tokio::test]
    async fn create_update_delete_round_trip() {
        let pool = temp_pool("hashtag-set-repo").await;
        let (workspace_id, _source_id) = seed_workspace_and_source(&pool).await;
        let repo = SqliteHashtagSetRepository::new(pool);

        let mut set = HashtagSet::new(
            workspace_id,
            None,
            None,
            "Football BR",
            vec!["#futebol".to_string(), "#shorts".to_string()],
        );
        repo.create(&set).await.unwrap();

        let listed = repo.list_for_workspace(workspace_id).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].hashtags.len(), 2);

        set.hashtags.push("#brasileirao".to_string());
        repo.update(&set).await.unwrap();
        let reloaded = repo.get(set.id).await.unwrap().unwrap();
        assert_eq!(reloaded.hashtags.len(), 3);

        repo.delete(set.id).await.unwrap();
        assert!(repo.get(set.id).await.unwrap().is_none());
    }
}
