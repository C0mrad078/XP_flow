use tauri::State;
use uuid::Uuid;

use crate::application::provider_configuration_health_service::ProviderConfigurationHealth;
use crate::domain::oauth::AuthFlowState;
use crate::domain::platform::Platform;
use crate::domain::platform_account::PlatformAccount;
use crate::error::AppError;
use crate::state::AppState;

/// Backend-authoritative connect-readiness for every platform (fix for
/// the Connect Account UX: the frontend never filters the provider list
/// itself — it always renders YouTube/TikTok/Kwai and uses this to pick
/// each card's state/button). Infallible by construction: an unreachable
/// broker is a real `BrokerUnavailable` status here, never a command
/// error, so the dialog is never left empty because this call "failed."
#[tauri::command]
pub async fn get_provider_configuration_health(
    state: State<'_, AppState>,
) -> Result<Vec<ProviderConfigurationHealth>, AppError> {
    Ok(state
        .provider_configuration_health_service
        .check_all()
        .await)
}

#[tauri::command]
pub async fn begin_platform_connect(
    state: State<'_, AppState>,
    workspace_id: Uuid,
    channel_id: Uuid,
    platform: Platform,
) -> Result<Uuid, AppError> {
    Ok(state
        .platform_auth_service
        .begin_connect(workspace_id, channel_id, platform)
        .await?)
}

#[tauri::command]
pub async fn begin_platform_reconnect(
    state: State<'_, AppState>,
    account_id: Uuid,
    allow_identity_change: bool,
) -> Result<Uuid, AppError> {
    Ok(state
        .platform_auth_service
        .begin_reconnect(account_id, allow_identity_change)
        .await?)
}

#[tauri::command]
pub async fn poll_platform_connect_status(
    state: State<'_, AppState>,
    session_id: Uuid,
) -> Result<Option<AuthFlowState>, AppError> {
    Ok(state.platform_auth_service.poll(session_id).await)
}

#[tauri::command]
pub async fn cancel_platform_connect(
    state: State<'_, AppState>,
    session_id: Uuid,
) -> Result<(), AppError> {
    state.platform_auth_service.cancel(session_id).await;
    Ok(())
}

#[tauri::command]
pub async fn validate_platform_account(
    state: State<'_, AppState>,
    account_id: Uuid,
) -> Result<PlatformAccount, AppError> {
    Ok(state.platform_auth_service.validate(account_id).await?)
}

#[tauri::command]
pub async fn refresh_platform_account(
    state: State<'_, AppState>,
    account_id: Uuid,
) -> Result<PlatformAccount, AppError> {
    Ok(state.platform_auth_service.refresh(account_id).await?)
}

#[tauri::command]
pub async fn disconnect_platform_account(
    state: State<'_, AppState>,
    account_id: Uuid,
) -> Result<PlatformAccount, AppError> {
    Ok(state.platform_auth_service.disconnect(account_id).await?)
}
