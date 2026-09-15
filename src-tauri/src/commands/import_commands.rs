use std::path::PathBuf;

use tauri::State;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::error::AppError;
use crate::infrastructure::filesystem::is_supported_video_extension;
use crate::services::job_runner::ImportSummary;
use crate::state::AppState;

/// Manual "Import Videos" (section 11) and drag-and-drop (section 12) both
/// resolve to a flat list of file paths before calling this — the
/// frontend never distinguishes between the two beyond how the paths were
/// collected (native dialog vs. a drop event), matching section 12's "use
/// the same backend ingestion pipeline as all other import methods."
#[tauri::command]
pub async fn import_files(
    state: State<'_, AppState>,
    workspace_id: Uuid,
    paths: Vec<String>,
    channel_id: Option<Uuid>,
) -> Result<ImportSummary, AppError> {
    let manual_source = state
        .source_service
        .ensure_manual_import_source(workspace_id)
        .await?;
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();

    Ok(state
        .job_runner
        .import_paths(workspace_id, manual_source.id, channel_id, paths)
        .await)
}

/// "an entire folder" (section 11) — a one-off walk, not a persistent
/// watched source. Use the Folder Sources flow (section 53) instead when
/// the folder should be watched going forward.
#[tauri::command]
pub async fn import_folder(
    state: State<'_, AppState>,
    workspace_id: Uuid,
    folder_path: String,
    recursive: bool,
    channel_id: Option<Uuid>,
) -> Result<ImportSummary, AppError> {
    let manual_source = state
        .source_service
        .ensure_manual_import_source(workspace_id)
        .await?;

    let max_depth = if recursive { usize::MAX } else { 1 };
    // Section 99 quality review: don't walk a potentially large folder
    // synchronously on the async command's thread.
    let paths: Vec<PathBuf> = tokio::task::spawn_blocking(move || {
        WalkDir::new(&folder_path)
            .max_depth(max_depth)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file() && is_supported_video_extension(e.path()))
            .map(|e| e.into_path())
            .collect()
    })
    .await
    .unwrap_or_default();

    Ok(state
        .job_runner
        .import_paths(workspace_id, manual_source.id, channel_id, paths)
        .await)
}
