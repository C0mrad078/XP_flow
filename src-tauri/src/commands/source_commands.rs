use tauri::State;
use uuid::Uuid;

use crate::application::source_service::{CreateSourceInput, SourceSummary, UpdateSourceInput};
use crate::domain::video_source::VideoSource;
use crate::error::AppError;
use crate::services::job_runner::ReconcileSummary;
use crate::state::AppState;

#[tauri::command]
pub async fn list_sources(
    state: State<'_, AppState>,
    workspace_id: Uuid,
) -> Result<Vec<SourceSummary>, AppError> {
    Ok(state.source_service.list_with_summary(workspace_id).await?)
}

#[tauri::command]
pub async fn create_source(
    state: State<'_, AppState>,
    workspace_id: Uuid,
    input: CreateSourceInput,
) -> Result<VideoSource, AppError> {
    let source = state
        .source_service
        .create_folder_source(workspace_id, input)
        .await?;
    if source.enabled && source.watch_enabled {
        state.job_runner.start_source_watch(&source);
    }

    // "Start indexing" (section 53, step 5) — index whatever already
    // exists in the folder right away rather than waiting for the next
    // filesystem event or periodic scan.
    let job_runner = state.job_runner.clone();
    let source_id = source.id;
    tauri::async_runtime::spawn(async move {
        let _ = job_runner.reconcile_source(source_id).await;
    });

    Ok(source)
}

#[tauri::command]
pub async fn update_source(
    state: State<'_, AppState>,
    id: Uuid,
    input: UpdateSourceInput,
) -> Result<VideoSource, AppError> {
    let previous = state.source_service.get(id).await?;
    let source = state.source_service.update_source(id, input).await?;

    if let Some(previous) = previous {
        state.job_runner.stop_source_watch(&previous);
    }
    if source.enabled && source.watch_enabled {
        state.job_runner.start_source_watch(&source);
    }

    Ok(source)
}

#[tauri::command]
pub async fn delete_source(state: State<'_, AppState>, id: Uuid) -> Result<(), AppError> {
    if let Some(source) = state.source_service.get(id).await? {
        state.job_runner.stop_source_watch(&source);
    }
    Ok(state.source_service.delete_source(id).await?)
}

#[tauri::command]
pub async fn scan_source_now(
    state: State<'_, AppState>,
    id: Uuid,
) -> Result<ReconcileSummary, AppError> {
    Ok(state.job_runner.reconcile_source(id).await?)
}
