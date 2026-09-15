use chrono::NaiveDate;
use tauri::State;
use uuid::Uuid;

use crate::domain::platform::Platform;
use crate::domain::schedule_exception::ScheduleException;
use crate::domain::schedule_slot::ScheduleSlot;
use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub async fn list_schedule_slots(
    state: State<'_, AppState>,
    channel_id: Uuid,
) -> Result<Vec<ScheduleSlot>, AppError> {
    Ok(state
        .schedule_slot_service
        .list_for_channel(channel_id)
        .await?)
}

#[tauri::command]
pub async fn create_schedule_slot(
    state: State<'_, AppState>,
    channel_id: Uuid,
    platform: Option<Platform>,
    day_of_week: i32,
    time_of_day: String,
) -> Result<ScheduleSlot, AppError> {
    Ok(state
        .schedule_slot_service
        .create_slot(channel_id, platform, day_of_week, time_of_day)
        .await?)
}

#[tauri::command]
pub async fn set_schedule_slot_active(
    state: State<'_, AppState>,
    id: Uuid,
    is_active: bool,
) -> Result<ScheduleSlot, AppError> {
    Ok(state
        .schedule_slot_service
        .set_slot_active(id, is_active)
        .await?)
}

#[tauri::command]
pub async fn delete_schedule_slot(state: State<'_, AppState>, id: Uuid) -> Result<(), AppError> {
    Ok(state.schedule_slot_service.delete_slot(id).await?)
}

#[tauri::command]
pub async fn copy_schedule_day(
    state: State<'_, AppState>,
    channel_id: Uuid,
    from_day: i32,
    to_days: Vec<i32>,
) -> Result<Vec<ScheduleSlot>, AppError> {
    Ok(state
        .schedule_slot_service
        .copy_day(channel_id, from_day, to_days)
        .await?)
}

#[tauri::command]
pub async fn add_schedule_exception(
    state: State<'_, AppState>,
    channel_id: Uuid,
    date: NaiveDate,
    reason: Option<String>,
) -> Result<ScheduleException, AppError> {
    Ok(state
        .schedule_slot_service
        .add_skip_exception(channel_id, date, reason)
        .await?)
}

#[tauri::command]
pub async fn remove_schedule_exception(
    state: State<'_, AppState>,
    id: Uuid,
) -> Result<(), AppError> {
    Ok(state.schedule_slot_service.remove_exception(id).await?)
}

#[tauri::command]
pub async fn list_schedule_exceptions(
    state: State<'_, AppState>,
    channel_id: Uuid,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<ScheduleException>, AppError> {
    Ok(state
        .schedule_slot_service
        .list_exceptions_in_range(channel_id, from, to)
        .await?)
}
