use tauri::State;
use uuid::Uuid;

use crate::application::provider_rate_limit_service::ProviderRateLimitStatus;
use crate::domain::publication::Publication;
use crate::domain::publishing::{ApprovalSource, PublicationAttempt};
use crate::domain::readiness::ReadinessIssue;
use crate::error::AppError;
use crate::state::AppState;

/// Forces an otherwise-eligible publication (not locked, its channel not
/// paused) to execute immediately instead of waiting for its scheduled
/// time — the "Publish Now" action. Goes through the exact same
/// claim-then-execute path as the periodic scan (section 85), so it
/// carries the same exactly-once guarantee.
#[tauri::command]
pub async fn publish_now(state: State<'_, AppState>, publication_id: Uuid) -> Result<(), AppError> {
    Ok(state
        .publishing_engine_service
        .publish_now(publication_id)
        .await?)
}

/// Manually retries a `Failed` publication right away rather than waiting
/// for its next backoff window — reuses `publish_now`'s same forced-
/// schedule-then-claim path, since both are "run this immediately"
/// requests distinguished only by the publication's current status.
#[tauri::command]
pub async fn retry_publication(
    state: State<'_, AppState>,
    publication_id: Uuid,
) -> Result<(), AppError> {
    Ok(state
        .publishing_engine_service
        .retry_publication(publication_id)
        .await?)
}

#[tauri::command]
pub async fn reconcile_publication(
    state: State<'_, AppState>,
    publication_id: Uuid,
) -> Result<String, AppError> {
    Ok(state
        .publishing_engine_service
        .reconcile_publication(publication_id)
        .await?)
}

#[tauri::command]
pub async fn create_publication_repost(
    state: State<'_, AppState>,
    publication_id: Uuid,
) -> Result<Publication, AppError> {
    Ok(state
        .publishing_engine_service
        .create_repost(publication_id)
        .await?)
}

/// Durable, never-overwritten per-attempt history for the Publication
/// Details drawer (section 90/94) — newest first.
#[tauri::command]
pub async fn get_publication_attempts(
    state: State<'_, AppState>,
    publication_id: Uuid,
) -> Result<Vec<PublicationAttempt>, AppError> {
    Ok(state
        .publishing_engine_service
        .get_attempts(publication_id)
        .await)
}

/// Why a publication is not (yet) ready to actually go out, computed
/// fresh from current account/video/consent state (section 74/75) — an
/// empty list means it's fully ready.
#[tauri::command]
pub async fn get_publication_readiness(
    state: State<'_, AppState>,
    publication_id: Uuid,
) -> Result<Vec<ReadinessIssue>, AppError> {
    Ok(state
        .publishing_readiness_service
        .compute(publication_id)
        .await?)
}

/// Records the user's explicit approval of a publication's *current*
/// rendered metadata (section 31-34) — the only way TikTok's
/// express-consent gate is ever satisfied. `approval_source` records how
/// the approval was given (e.g. confirmed while adding to the queue vs.
/// a later bulk approval) for audit purposes only; it doesn't change
/// what the consent covers.
#[tauri::command]
pub async fn record_publication_consent(
    state: State<'_, AppState>,
    publication_id: Uuid,
    approval_source: ApprovalSource,
) -> Result<(), AppError> {
    Ok(state
        .publishing_engine_service
        .record_consent(publication_id, approval_source)
        .await?)
}

/// This account's currently-known rate-limit state per operation class
/// (section 23/59) — an empty `limited_until` means "not currently
/// limited," not "never limited."
#[tauri::command]
pub async fn get_provider_rate_state(
    state: State<'_, AppState>,
    platform_account_id: Uuid,
) -> Result<Vec<ProviderRateLimitStatus>, AppError> {
    Ok(state
        .provider_rate_limit_service
        .get_state(platform_account_id)
        .await)
}
