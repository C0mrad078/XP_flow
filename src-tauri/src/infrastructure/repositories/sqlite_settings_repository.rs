use async_trait::async_trait;
use chrono::Utc;
use sqlx::{Row, SqlitePool};

use crate::domain::app_settings::AppSettings;
use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::ports::repositories::SettingsRepository;

const SETTINGS_KEY: &str = "app_settings";

pub struct SqliteSettingsRepository {
    pool: SqlitePool,
}

impl SqliteSettingsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn map_repo_err(err: sqlx::Error) -> DomainError {
    DomainError::Repository(err.to_string())
}

#[async_trait]
impl SettingsRepository for SqliteSettingsRepository {
    async fn get(&self) -> DomainResult<AppSettings> {
        let row = sqlx::query("SELECT value FROM app_settings WHERE key = ?")
            .bind(SETTINGS_KEY)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_repo_err)?;

        match row {
            Some(row) => {
                let value: String = row.try_get("value").map_err(map_repo_err)?;
                serde_json::from_str(&value)
                    .map_err(|e| DomainError::Repository(format!("corrupt settings payload: {e}")))
            }
            None => Ok(AppSettings::default()),
        }
    }

    async fn save(&self, settings: &AppSettings) -> DomainResult<()> {
        let value = serde_json::to_string(settings)
            .map_err(|e| DomainError::Validation(format!("failed to serialize settings: {e}")))?;

        sqlx::query(
            "INSERT INTO app_settings (key, value, updated_at) VALUES (?, ?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        )
        .bind(SETTINGS_KEY)
        .bind(value)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::app_settings::ThemePreference;
    use crate::persistence::db::init_pool;

    async fn test_pool() -> SqlitePool {
        let dir =
            std::env::temp_dir().join(format!("xpflow-settings-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        init_pool(&dir.join("test.db")).await.unwrap()
    }

    #[tokio::test]
    async fn get_returns_default_when_unset() {
        let repo = SqliteSettingsRepository::new(test_pool().await);
        let settings = repo.get().await.unwrap();
        assert_eq!(settings.theme, ThemePreference::Dark);
    }

    #[tokio::test]
    async fn save_then_get_round_trips_and_upserts() {
        let repo = SqliteSettingsRepository::new(test_pool().await);

        let mut settings = AppSettings {
            theme: ThemePreference::Light,
            ..Default::default()
        };
        repo.save(&settings).await.unwrap();
        assert_eq!(repo.get().await.unwrap().theme, ThemePreference::Light);

        settings.theme = ThemePreference::System;
        repo.save(&settings).await.unwrap();
        assert_eq!(repo.get().await.unwrap().theme, ThemePreference::System);
    }
}
