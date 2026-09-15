use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::ports::repositories::WorkspaceRepository;
use crate::domain::workspace::Workspace;

use super::parse_dt;

pub struct SqliteWorkspaceRepository {
    pool: SqlitePool,
}

impl SqliteWorkspaceRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn map_repo_err(err: sqlx::Error) -> DomainError {
    DomainError::Repository(err.to_string())
}

fn row_to_workspace(row: &sqlx::sqlite::SqliteRow) -> Result<Workspace, sqlx::Error> {
    let id: String = row.try_get("id")?;
    let created_at: String = row.try_get("created_at")?;
    let updated_at: String = row.try_get("updated_at")?;
    Ok(Workspace {
        id: Uuid::parse_str(&id).unwrap_or_default(),
        name: row.try_get("name")?,
        created_at: parse_dt(&created_at),
        updated_at: parse_dt(&updated_at),
    })
}

#[async_trait]
impl WorkspaceRepository for SqliteWorkspaceRepository {
    async fn create(&self, workspace: &Workspace) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO workspaces (id, name, created_at, updated_at) VALUES (?, ?, ?, ?)",
        )
        .bind(workspace.id.to_string())
        .bind(&workspace.name)
        .bind(workspace.created_at.to_rfc3339())
        .bind(workspace.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn get(&self, id: Uuid) -> DomainResult<Option<Workspace>> {
        let row =
            sqlx::query("SELECT id, name, created_at, updated_at FROM workspaces WHERE id = ?")
                .bind(id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(map_repo_err)?;

        row.map(|r| row_to_workspace(&r).map_err(map_repo_err))
            .transpose()
    }

    async fn list(&self) -> DomainResult<Vec<Workspace>> {
        let rows = sqlx::query(
            "SELECT id, name, created_at, updated_at FROM workspaces ORDER BY created_at ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_repo_err)?;

        rows.iter()
            .map(|r| row_to_workspace(r).map_err(map_repo_err))
            .collect()
    }

    async fn get_current(&self) -> DomainResult<Option<Workspace>> {
        let row = sqlx::query(
            "SELECT id, name, created_at, updated_at FROM workspaces ORDER BY created_at DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_repo_err)?;

        row.map(|r| row_to_workspace(&r).map_err(map_repo_err))
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::db::init_pool;

    async fn test_pool() -> SqlitePool {
        let dir = std::env::temp_dir().join(format!("xpflow-repo-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        init_pool(&dir.join("test.db")).await.unwrap()
    }

    #[tokio::test]
    async fn create_then_get_current_round_trips() {
        let pool = test_pool().await;
        let repo = SqliteWorkspaceRepository::new(pool);

        let workspace = Workspace::new("My Workspace");
        repo.create(&workspace).await.unwrap();

        let current = repo.get_current().await.unwrap().unwrap();
        assert_eq!(current.id, workspace.id);
        assert_eq!(current.name, "My Workspace");
    }

    #[tokio::test]
    async fn get_current_is_none_when_empty() {
        let pool = test_pool().await;
        let repo = SqliteWorkspaceRepository::new(pool);
        assert!(repo.get_current().await.unwrap().is_none());
    }
}
