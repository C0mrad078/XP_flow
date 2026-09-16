use tauri::State;

use crate::application::settings_service::{UpdatePublishingSettingsInput, UpdateSettingsInput};
use crate::domain::app_settings::{AppSettings, PublishingSettings};
use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, AppError> {
    Ok(state.settings_service.get().await?)
}

#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    input: UpdateSettingsInput,
) -> Result<AppSettings, AppError> {
    Ok(state.settings_service.update(input).await?)
}

/// A dedicated, narrower read/write pair for Settings → Publishing
/// (Phase 5.1 section 24/55) — lets the frontend query and invalidate
/// just this slice of settings instead of the whole `AppSettings`
/// object every time a publishing control changes.
#[tauri::command]
pub async fn get_publishing_settings(
    state: State<'_, AppState>,
) -> Result<PublishingSettings, AppError> {
    Ok(state.settings_service.get().await?.publishing)
}

#[tauri::command]
pub async fn update_publishing_settings(
    state: State<'_, AppState>,
    input: UpdatePublishingSettingsInput,
) -> Result<PublishingSettings, AppError> {
    Ok(state
        .settings_service
        .update_publishing(input)
        .await?
        .publishing)
}

/// Thin, explicit wrappers around `update_publishing_settings` for the
/// single most common action (section 58) — a dedicated command instead
/// of expecting the frontend to construct a partial-update payload for
/// something this frequent.
#[tauri::command]
pub async fn pause_publishing(state: State<'_, AppState>) -> Result<PublishingSettings, AppError> {
    Ok(state
        .settings_service
        .set_publishing_paused(true)
        .await?
        .publishing)
}

#[tauri::command]
pub async fn resume_publishing(state: State<'_, AppState>) -> Result<PublishingSettings, AppError> {
    Ok(state
        .settings_service
        .set_publishing_paused(false)
        .await?
        .publishing)
}
