use tauri::State;
use uuid::Uuid;

use crate::domain::channel::Channel;
use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub async fn list_channels(
    state: State<'_, AppState>,
    workspace_id: Uuid,
) -> Result<Vec<Channel>, AppError> {
    Ok(state.channel_service.list(workspace_id).await?)
}

#[tauri::command]
pub async fn create_channel(
    state: State<'_, AppState>,
    workspace_id: Uuid,
    name: String,
) -> Result<Channel, AppError> {
    Ok(state.channel_service.create(workspace_id, name).await?)
}
