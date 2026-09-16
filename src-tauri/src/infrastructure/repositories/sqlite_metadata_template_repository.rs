use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::platform::Platform;
use crate::domain::ports::repositories::MetadataTemplateRepository;
use crate::domain::publishing::{MetadataTemplate, TemplateKind};

use super::parse_dt;

pub struct SqliteMetadataTemplateRepository {
    pool: SqlitePool,
}

impl SqliteMetadataTemplateRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn map_repo_err(err: sqlx::Error) -> DomainError {
    DomainError::Repository(err.to_string())
}

const SELECT_COLUMNS: &str =
    "id, workspace_id, channel_id, platform, kind, template_text, created_at, updated_at";

fn row_to_template(row: &sqlx::sqlite::SqliteRow) -> Result<MetadataTemplate, DomainError> {
    let channel_id: Option<String> = row.try_get("channel_id").map_err(map_repo_err)?;
    let platform: Option<String> = row.try_get("platform").map_err(map_repo_err)?;

    Ok(MetadataTemplate {
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
        kind: row
            .try_get::<String, _>("kind")
            .map_err(map_repo_err)?
            .parse::<TemplateKind>()
            .map_err(DomainError::Validation)?,
        template_text: row.try_get("template_text").map_err(map_repo_err)?,
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
impl MetadataTemplateRepository for SqliteMetadataTemplateRepository {
    async fn create(&self, template: &MetadataTemplate) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO metadata_templates (id, workspace_id, channel_id, platform, kind, template_text, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(template.id.to_string())
        .bind(template.workspace_id.to_string())
        .bind(template.channel_id.map(|id| id.to_string()))
        .bind(template.platform.map(|p| p.as_str()))
        .bind(template.kind.as_str())
        .bind(&template.template_text)
        .bind(template.created_at.to_rfc3339())
        .bind(template.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn update(&self, template: &MetadataTemplate) -> DomainResult<()> {
        sqlx::query(
            "UPDATE metadata_templates SET channel_id = ?, platform = ?, kind = ?, template_text = ?, updated_at = ? WHERE id = ?",
        )
        .bind(template.channel_id.map(|id| id.to_string()))
        .bind(template.platform.map(|p| p.as_str()))
        .bind(template.kind.as_str())
        .bind(&template.template_text)
        .bind(template.updated_at.to_rfc3339())
        .bind(template.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> DomainResult<()> {
        sqlx::query("DELETE FROM metadata_templates WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(map_repo_err)?;
        Ok(())
    }

    async fn get(&self, id: Uuid) -> DomainResult<Option<MetadataTemplate>> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM metadata_templates WHERE id = ?"
        ))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_repo_err)?;
        row.as_ref().map(row_to_template).transpose()
    }

    async fn list_for_workspace(&self, workspace_id: Uuid) -> DomainResult<Vec<MetadataTemplate>> {
        let rows = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM metadata_templates WHERE workspace_id = ? ORDER BY created_at ASC"
        ))
        .bind(workspace_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(map_repo_err)?;
        rows.iter().map(row_to_template).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{seed_channel, seed_workspace_and_source, temp_pool};

    #[tokio::test]
    async fn create_update_delete_round_trip() {
        let pool = temp_pool("metadata-template-repo").await;
        let (workspace_id, _source_id) = seed_workspace_and_source(&pool).await;
        let repo = SqliteMetadataTemplateRepository::new(pool);

        let mut template = MetadataTemplate::new(
            workspace_id,
            None,
            None,
            TemplateKind::Title,
            "{title} | {channel}",
        );
        repo.create(&template).await.unwrap();

        let listed = repo.list_for_workspace(workspace_id).await.unwrap();
        assert_eq!(listed.len(), 1);

        template.template_text = "{title} on {platform}".to_string();
        repo.update(&template).await.unwrap();
        let reloaded = repo.get(template.id).await.unwrap().unwrap();
        assert_eq!(reloaded.template_text, "{title} on {platform}");

        repo.delete(template.id).await.unwrap();
        assert!(repo.get(template.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn scoped_templates_persist_channel_and_platform() {
        let pool = temp_pool("metadata-template-repo-scoped").await;
        let (workspace_id, _source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Channel").await;
        let repo = SqliteMetadataTemplateRepository::new(pool);

        let template = MetadataTemplate::new(
            workspace_id,
            Some(channel_id),
            Some(Platform::TikTok),
            TemplateKind::Description,
            "{hashtags}",
        );
        repo.create(&template).await.unwrap();

        let reloaded = repo.get(template.id).await.unwrap().unwrap();
        assert_eq!(reloaded.channel_id, Some(channel_id));
        assert_eq!(reloaded.platform, Some(Platform::TikTok));
    }
}
