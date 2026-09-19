use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::application::publication_service::AddToQueueOutcome;
use crate::domain::platform::Platform;
use crate::domain::publication::{Publication, PublicationStatus};
use crate::domain::publication_query::{PublicationListQuery, PublicationPage, QueueSort};
use crate::domain::video_status::VideoPriority;
use crate::error::AppError;
use crate::state::AppState;

/// Wire shape for `list_publications` — kept separate from the domain
/// `PublicationListQuery` so IPC (de)serialization concerns don't leak
/// into the domain type (mirrors `ContentListRequest`).
#[derive(Debug, Deserialize)]
pub struct PublicationListRequest {
    pub search: Option<String>,
    pub channel_id: Option<Uuid>,
    pub platform_account_id: Option<Uuid>,
    pub platform: Option<Platform>,
    pub priority: Option<VideoPriority>,
    pub statuses: Option<Vec<PublicationStatus>>,
    #[serde(default)]
    pub requires_attention: bool,
    pub sort: Option<QueueSort>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

fn to_domain_query(workspace_id: Uuid, request: PublicationListRequest) -> PublicationListQuery {
    let mut query = PublicationListQuery::new(workspace_id);
    query.search = request.search;
    query.channel_id = request.channel_id;
    query.platform_account_id = request.platform_account_id;
    query.platform = request.platform;
    query.priority = request.priority;
    query.statuses = request.statuses;
    query.requires_attention = request.requires_attention;
    query.sort = request.sort.unwrap_or(QueueSort::QueueOrder);
    query.page = request.page.unwrap_or(0).max(0);
    query.page_size = request.page_size.unwrap_or(60).clamp(1, 500);
    query
}

#[derive(Debug, Serialize)]
pub struct AddToQueueOutcomeDto {
    pub video_id: Uuid,
    pub publication: Option<Publication>,
    pub error: Option<AppError>,
}

impl From<AddToQueueOutcome> for AddToQueueOutcomeDto {
    fn from(outcome: AddToQueueOutcome) -> Self {
        match outcome.result {
            Ok(publication) => Self {
                video_id: outcome.video_id,
                publication: Some(publication),
                error: None,
            },
            Err(err) => Self {
                video_id: outcome.video_id,
                publication: None,
                error: Some(err.into()),
            },
        }
    }
}

#[tauri::command]
pub async fn list_publications(
    state: State<'_, AppState>,
    workspace_id: Uuid,
    request: PublicationListRequest,
) -> Result<PublicationPage, AppError> {
    Ok(state
        .publication_service
        .list(to_domain_query(workspace_id, request))
        .await?)
}

#[tauri::command]
pub async fn get_publication(
    state: State<'_, AppState>,
    id: Uuid,
) -> Result<Option<Publication>, AppError> {
    Ok(state.publication_service.get(id).await?)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn add_to_queue(
    state: State<'_, AppState>,
    workspace_id: Uuid,
    video_id: Uuid,
    channel_id: Uuid,
    platform: Platform,
    platform_account_id: Option<Uuid>,
    priority: Option<VideoPriority>,
) -> Result<Publication, AppError> {
    Ok(state
        .publication_service
        .add_to_queue(
            workspace_id,
            video_id,
            channel_id,
            platform,
            platform_account_id,
            priority,
        )
        .await?)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn add_to_queue_bulk(
    state: State<'_, AppState>,
    workspace_id: Uuid,
    video_ids: Vec<Uuid>,
    channel_id: Uuid,
    platform: Platform,
    platform_account_id: Option<Uuid>,
    priority: Option<VideoPriority>,
) -> Result<Vec<AddToQueueOutcomeDto>, AppError> {
    let outcomes = state
        .publication_service
        .add_to_queue_bulk(
            workspace_id,
            video_ids,
            channel_id,
            platform,
            platform_account_id,
            priority,
        )
        .await;
    Ok(outcomes.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub async fn cancel_publication(
    state: State<'_, AppState>,
    id: Uuid,
) -> Result<Publication, AppError> {
    Ok(state.publication_service.cancel(id).await?)
}

#[tauri::command]
pub async fn archive_publication(
    state: State<'_, AppState>,
    id: Uuid,
) -> Result<Publication, AppError> {
    Ok(state.publication_service.archive(id).await?)
}

#[tauri::command]
pub async fn set_publication_priority(
    state: State<'_, AppState>,
    id: Uuid,
    priority: VideoPriority,
) -> Result<Publication, AppError> {
    Ok(state.publication_service.set_priority(id, priority).await?)
}

#[tauri::command]
pub async fn set_publication_locked(
    state: State<'_, AppState>,
    id: Uuid,
    locked: bool,
) -> Result<Publication, AppError> {
    Ok(state.publication_service.set_locked(id, locked).await?)
}

#[tauri::command]
pub async fn reorder_queue(
    state: State<'_, AppState>,
    ordered_publication_ids: Vec<Uuid>,
) -> Result<(), AppError> {
    Ok(state
        .publication_service
        .reorder_queue(ordered_publication_ids)
        .await?)
}
