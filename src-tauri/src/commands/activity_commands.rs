use tauri::State;

use crate::domain::activity_event::ActivityEvent;
use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub async fn list_recent_activity(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<ActivityEvent>, AppError> {
    Ok(state
        .activity_service
        .list_recent(limit.unwrap_or(100))
        .await?)
}
