use tauri::State;

use crate::domain::workspace::Workspace;
use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub async fn get_current_workspace(
    state: State<'_, AppState>,
) -> Result<Option<Workspace>, AppError> {
    Ok(state.workspace_service.get_current().await?)
}

#[tauri::command]
pub async fn create_workspace(
    state: State<'_, AppState>,
    name: String,
) -> Result<Workspace, AppError> {
    let workspace = state.workspace_service.create_workspace(name).await?;
    // Every workspace needs its implicit Manual Import source before the
    // Content page's "Import Videos" / drag-and-drop can be used.
    state
        .source_service
        .ensure_manual_import_source(workspace.id)
        .await?;
    Ok(workspace)
}
