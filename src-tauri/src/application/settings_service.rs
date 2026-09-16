use std::sync::Arc;

use serde::Deserialize;

use crate::domain::app_settings::{AppSettings, MissedSchedulePolicy, ThemePreference};
use crate::domain::errors::DomainResult;
use crate::domain::ports::repositories::SettingsRepository;

/// Partial update payload from the Settings screen — only fields the user
/// actually changed are sent, so unrelated settings are left untouched.
#[derive(Debug, Deserialize)]
pub struct UpdateSettingsInput {
    pub theme: Option<ThemePreference>,
    pub launch_on_startup: Option<bool>,
}

/// Partial update payload for Settings → Publishing (Phase 5.1 section
/// 55) — same "only what changed" discipline as [`UpdateSettingsInput`].
#[derive(Debug, Deserialize, Default)]
pub struct UpdatePublishingSettingsInput {
    pub enabled: Option<bool>,
    pub paused: Option<bool>,
    pub max_concurrent_uploads: Option<u32>,
    pub missed_schedule_policy: Option<MissedSchedulePolicy>,
    pub missed_schedule_grace_period_minutes: Option<u32>,
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

    pub async fn update_publishing(
        &self,
        input: UpdatePublishingSettingsInput,
    ) -> DomainResult<AppSettings> {
        let mut settings = self.repo.get().await?;
        if let Some(enabled) = input.enabled {
            settings.publishing.enabled = enabled;
        }
        if let Some(paused) = input.paused {
            settings.publishing.paused = paused;
        }
        if let Some(max_concurrent_uploads) = input.max_concurrent_uploads {
            settings.publishing.max_concurrent_uploads = max_concurrent_uploads;
        }
        if let Some(policy) = input.missed_schedule_policy {
            settings.publishing.missed_schedule_policy = policy;
        }
        if let Some(grace) = input.missed_schedule_grace_period_minutes {
            settings.publishing.missed_schedule_grace_period_minutes = grace;
        }
        settings.publishing = settings.publishing.clone().clamp();

        self.repo.save(&settings).await?;
        Ok(settings)
    }

    pub async fn set_publishing_paused(&self, paused: bool) -> DomainResult<AppSettings> {
        self.update_publishing(UpdatePublishingSettingsInput {
            paused: Some(paused),
            ..Default::default()
        })
        .await
    }
}
