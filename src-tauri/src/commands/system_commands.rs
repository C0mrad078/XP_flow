use serde::Serialize;
use tauri::State;

use crate::domain::ports::media_service::MediaToolchainStatus;
use crate::error::AppError;
use crate::state::AppState;

/// Drives the Settings → About panel (section 37): version, OS, and where
/// XP FLOW keeps its data/logs/database on this machine.
#[derive(Debug, Serialize)]
pub struct AppInfo {
    pub version: String,
    pub os: String,
    pub arch: String,
    pub data_dir: String,
    pub log_dir: String,
    pub database_path: String,
}

#[tauri::command]
pub async fn get_app_info(state: State<'_, AppState>) -> Result<AppInfo, AppError> {
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        data_dir: state.paths.data_dir.display().to_string(),
        log_dir: state.paths.log_dir.display().to_string(),
        database_path: state.paths.database_path().display().to_string(),
    })
}

#[tauri::command]
pub async fn get_media_status(
    state: State<'_, AppState>,
) -> Result<MediaToolchainStatus, AppError> {
    Ok(state.media_status_service.status().await)
}
