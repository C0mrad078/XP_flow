use serde::Serialize;
use tauri::State;

use crate::domain::ports::media_service::MediaToolchainStatus;
use crate::error::AppError;
use crate::infrastructure::filesystem::{clear_dir_contents, dir_size_bytes};
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

/// Backs Settings → Storage's cache accounting (section 67).
#[derive(Debug, Serialize)]
pub struct CacheInfo {
    pub database_size_bytes: u64,
    pub thumbnail_cache_size_bytes: u64,
    pub temp_cache_size_bytes: u64,
}

#[tauri::command]
pub async fn get_cache_info(state: State<'_, AppState>) -> Result<CacheInfo, AppError> {
    let database_size_bytes = tokio::fs::metadata(state.paths.database_path())
        .await
        .map(|m| m.len())
        .unwrap_or(0);
    let thumbnail_cache_size_bytes = dir_size_bytes(state.paths.thumbnail_cache_dir.clone()).await;
    let temp_cache_size_bytes = dir_size_bytes(state.paths.temp_cache_dir.clone()).await;

    Ok(CacheInfo {
        database_size_bytes,
        thumbnail_cache_size_bytes,
        temp_cache_size_bytes,
    })
}

/// Only ever clears `cache/temp/` — thumbnails are never wiped by this
/// (section 67: "Do not clear useful thumbnails without confirmation").
#[tauri::command]
pub async fn clear_temp_cache(state: State<'_, AppState>) -> Result<u64, AppError> {
    clear_dir_contents(state.paths.temp_cache_dir.clone())
        .await
        .map_err(|e| {
            AppError::new(
                crate::error::ErrorCode::Internal,
                "Couldn't clear the temporary cache.",
                e.to_string(),
            )
        })
}
