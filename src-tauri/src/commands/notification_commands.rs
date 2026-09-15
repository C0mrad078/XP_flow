use tauri::State;
use uuid::Uuid;

use crate::domain::notification::Notification;
use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub async fn list_recent_notifications(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<Notification>, AppError> {
    Ok(state
        .notification_service
        .list_recent(limit.unwrap_or(50))
        .await?)
}

#[tauri::command]
pub async fn unread_notification_count(state: State<'_, AppState>) -> Result<i64, AppError> {
    Ok(state.notification_service.unread_count().await?)
}

#[tauri::command]
pub async fn mark_notification_read(state: State<'_, AppState>, id: Uuid) -> Result<(), AppError> {
    Ok(state.notification_service.mark_read(id).await?)
}
