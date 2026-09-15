use tauri::State;

use crate::application::settings_service::UpdateSettingsInput;
use crate::domain::app_settings::AppSettings;
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
