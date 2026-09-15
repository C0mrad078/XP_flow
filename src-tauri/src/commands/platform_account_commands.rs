use tauri::State;
use uuid::Uuid;

use crate::domain::platform::Platform;
use crate::domain::platform_account::PlatformAccount;
use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub async fn list_platform_accounts(
    state: State<'_, AppState>,
    channel_id: Uuid,
) -> Result<Vec<PlatformAccount>, AppError> {
    Ok(state
        .platform_account_service
        .list_for_channel(channel_id)
        .await?)
}

#[tauri::command]
pub async fn create_platform_account(
    state: State<'_, AppState>,
    channel_id: Uuid,
    platform: Platform,
) -> Result<PlatformAccount, AppError> {
    Ok(state
        .platform_account_service
        .create(channel_id, platform)
        .await?)
}

#[tauri::command]
pub async fn set_default_platform_account(
    state: State<'_, AppState>,
    id: Uuid,
) -> Result<PlatformAccount, AppError> {
    Ok(state.platform_account_service.set_default(id).await?)
}
