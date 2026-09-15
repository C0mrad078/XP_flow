use std::sync::Arc;

use serde::Deserialize;

use crate::domain::app_settings::{AppSettings, ThemePreference};
use crate::domain::errors::DomainResult;
use crate::domain::ports::repositories::SettingsRepository;

/// Partial update payload from the Settings screen — only fields the user
/// actually changed are sent, so unrelated settings are left untouched.
#[derive(Debug, Deserialize)]
pub struct UpdateSettingsInput {
    pub theme: Option<ThemePreference>,
    pub launch_on_startup: Option<bool>,
}

pub struct SettingsService {
    repo: Arc<dyn SettingsRepository>,
}

impl SettingsService {
    pub fn new(repo: Arc<dyn SettingsRepository>) -> Self {
        Self { repo }
    }

    pub async fn get(&self) -> DomainResult<AppSettings> {
        self.repo.get().await
    }

    pub async fn update(&self, input: UpdateSettingsInput) -> DomainResult<AppSettings> {
        let mut settings = self.repo.get().await?;

        if let Some(theme) = input.theme {
            settings.theme = theme;
        }
        if let Some(launch_on_startup) = input.launch_on_startup {
            settings.launch_on_startup = launch_on_startup;
        }

        self.repo.save(&settings).await?;
        Ok(settings)
    }
}
