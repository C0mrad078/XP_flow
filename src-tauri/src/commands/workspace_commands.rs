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
    Ok(state.workspace_service.create_workspace(name).await?)
}
