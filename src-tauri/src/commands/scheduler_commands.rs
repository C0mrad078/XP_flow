use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use tauri::State;
use uuid::Uuid;

use crate::application::scheduler_service::BulkScheduleResult;
use crate::domain::publication::Publication;
use crate::domain::publication_query::CalendarPublication;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct BulkScheduleResultDto {
    pub scheduled: Vec<Publication>,
    pub skipped_count: usize,
}

impl From<BulkScheduleResult> for BulkScheduleResultDto {
    fn from(result: BulkScheduleResult) -> Self {
        Self {
            scheduled: result.scheduled,
            skipped_count: result.skipped.len(),
        }
    }
}

#[tauri::command]
pub async fn schedule_publication(
    state: State<'_, AppState>,
    publication_id: Uuid,
    at: DateTime<Utc>,
) -> Result<Publication, AppError> {
    Ok(state
        .scheduler_service
        .schedule_at(publication_id, at)
        .await?)
}

#[tauri::command]
pub async fn unschedule_publication(
    state: State<'_, AppState>,
    publication_id: Uuid,
) -> Result<Publication, AppError> {
    Ok(state.scheduler_service.unschedule(publication_id).await?)
}

#[tauri::command]
pub async fn reschedule_publication_to_date(
    state: State<'_, AppState>,
    publication_id: Uuid,
    new_date: NaiveDate,
) -> Result<Publication, AppError> {
    Ok(state
        .scheduler_service
        .reschedule_to_date(publication_id, new_date)
        .await?)
}

#[tauri::command]
pub async fn auto_schedule_publication(
    state: State<'_, AppState>,
    publication_id: Uuid,
) -> Result<Publication, AppError> {
    Ok(state
        .scheduler_service
        .auto_schedule(publication_id)
        .await?)
}

#[tauri::command]
pub async fn auto_schedule_channel(
    state: State<'_, AppState>,
    channel_id: Uuid,
    horizon_days: Option<i64>,
) -> Result<BulkScheduleResultDto, AppError> {
    let horizon = horizon_days.unwrap_or(crate::domain::scheduling::DEFAULT_SEARCH_HORIZON_DAYS);
    Ok(state
        .scheduler_service
        .auto_schedule_channel(channel_id, horizon)
        .await?
        .into())
}

#[tauri::command]
pub async fn fill_schedule_gaps(
    state: State<'_, AppState>,
    channel_id: Uuid,
) -> Result<BulkScheduleResultDto, AppError> {
    Ok(state
        .scheduler_service
        .fill_schedule_gaps(channel_id)
        .await?
        .into())
}

#[tauri::command]
pub async fn rebuild_channel_schedule(
    state: State<'_, AppState>,
    channel_id: Uuid,
    horizon_days: Option<i64>,
) -> Result<BulkScheduleResultDto, AppError> {
    let horizon = horizon_days.unwrap_or(crate::domain::scheduling::DEFAULT_SEARCH_HORIZON_DAYS);
    Ok(state
        .scheduler_service
        .rebuild_channel_schedule(channel_id, horizon)
        .await?
        .into())
}

#[tauri::command]
pub async fn get_calendar_range(
    state: State<'_, AppState>,
    workspace_id: Uuid,
    channel_id: Option<Uuid>,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<Vec<CalendarPublication>, AppError> {
    Ok(state
        .scheduler_service
        .calendar_range(workspace_id, channel_id, start, end)
        .await?)
}

#[tauri::command]
pub async fn list_due_publications(
    state: State<'_, AppState>,
    workspace_id: Uuid,
) -> Result<Vec<Publication>, AppError> {
    Ok(state
        .scheduler_service
        .due_publications(workspace_id, Utc::now())
        .await?)
}
