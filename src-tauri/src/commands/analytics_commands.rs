use crate::domain::analytics::{AnalyticsCapabilities, PublicationMetricSnapshot};
use crate::domain::platform::Platform;
use crate::error::AppError;
use crate::state::AppState;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub async fn get_analytics_capabilities(
    platform: Platform,
) -> Result<AnalyticsCapabilities, AppError> {
    Ok(crate::application::analytics_service::AnalyticsService::capabilities(platform))
}

#[tauri::command]
pub async fn list_publication_analytics(
    state: State<'_, AppState>,
    publication_id: Uuid,
    days: Option<i64>,
) -> Result<Vec<PublicationMetricSnapshot>, AppError> {
    Ok(state
        .analytics_service
        .publication_snapshots(publication_id, days.unwrap_or(30))
        .await?)
}

#[tauri::command]
pub async fn sync_publication_analytics(
    state: State<'_, AppState>,
    publication_id: Uuid,
) -> Result<PublicationMetricSnapshot, AppError> {
    Ok(state
        .analytics_service
        .sync_publication(publication_id)
        .await?)
}

#[tauri::command]
pub async fn sync_workspace_analytics(
    state: State<'_, AppState>,
    workspace_id: Uuid,
) -> Result<usize, AppError> {
    Ok(state
        .analytics_service
        .sync_workspace(workspace_id, 20)
        .await?)
}
