use serde::Deserialize;
use tauri::State;
use uuid::Uuid;

use crate::application::content_service::{BulkUpdateInput, UpdateVideoInput, VideoDetail};
use crate::domain::video::Video;
use crate::domain::video_query::{
    ChannelFilter, DuplicateFilter, VideoLibrarySummary, VideoListQuery, VideoPage, VideoSort,
};
use crate::domain::video_status::{
    AvailabilityStatus, Orientation, ValidationStatus, VideoPriority,
};
use crate::error::AppError;
use crate::infrastructure::filesystem::reveal_in_file_manager;
use crate::state::AppState;

/// Wire shape the frontend sends for a list query — kept separate from
/// the domain `VideoListQuery` so IPC (de)serialization concerns don't
/// leak into the domain type.
#[derive(Debug, Deserialize)]
pub struct ContentListRequest {
    pub search: Option<String>,
    pub channel_id: Option<Uuid>,
    pub unassigned_only: Option<bool>,
    pub source_id: Option<Uuid>,
    pub validation_status: Option<ValidationStatus>,
    pub availability_status: Option<AvailabilityStatus>,
    pub orientation: Option<Orientation>,
    pub priority: Option<VideoPriority>,
    pub possible_duplicates_only: Option<bool>,
    pub include_archived: Option<bool>,
    pub sort: Option<VideoSort>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

fn to_domain_query(workspace_id: Uuid, request: ContentListRequest) -> VideoListQuery {
    let mut query = VideoListQuery::new(workspace_id);
    query.search = request.search;
    query.channel = if request.unassigned_only.unwrap_or(false) {
        Some(ChannelFilter::Unassigned)
    } else {
        request.channel_id.map(ChannelFilter::Any)
    };
    query.source_id = request.source_id;
    query.validation_status = request.validation_status;
    query.availability_status = request.availability_status;
    query.orientation = request.orientation;
    query.priority = request.priority;
    query.duplicate = request
        .possible_duplicates_only
        .unwrap_or(false)
        .then_some(DuplicateFilter::PossibleDuplicates);
    query.include_archived = request.include_archived.unwrap_or(false);
    query.sort = request.sort.unwrap_or(VideoSort::NewestImported);
    query.page = request.page.unwrap_or(0).max(0);
    query.page_size = request.page_size.unwrap_or(60).clamp(1, 500);
    query
}

#[tauri::command]
pub async fn list_content(
    state: State<'_, AppState>,
    workspace_id: Uuid,
    request: ContentListRequest,
) -> Result<VideoPage, AppError> {
    Ok(state
        .content_service
        .list(to_domain_query(workspace_id, request))
        .await?)
}

#[tauri::command]
pub async fn get_content_summary(
    state: State<'_, AppState>,
    workspace_id: Uuid,
) -> Result<VideoLibrarySummary, AppError> {
    Ok(state.content_service.summary(workspace_id).await?)
}

#[tauri::command]
pub async fn get_video_detail(
    state: State<'_, AppState>,
    id: Uuid,
) -> Result<Option<VideoDetail>, AppError> {
    Ok(state.content_service.get_detail(id).await?)
}

#[tauri::command]
pub async fn update_video(
    state: State<'_, AppState>,
    id: Uuid,
    input: UpdateVideoInput,
) -> Result<Video, AppError> {
    Ok(state.content_service.update(id, input).await?)
}

#[tauri::command]
pub async fn bulk_update_videos(
    state: State<'_, AppState>,
    ids: Vec<Uuid>,
    input: BulkUpdateInput,
) -> Result<usize, AppError> {
    Ok(state.content_service.bulk_update(&ids, input).await?)
}

#[tauri::command]
pub async fn set_video_archived(
    state: State<'_, AppState>,
    id: Uuid,
    archived: bool,
) -> Result<Video, AppError> {
    Ok(state.content_service.set_archived(id, archived).await?)
}

#[tauri::command]
pub async fn remove_video(state: State<'_, AppState>, id: Uuid) -> Result<(), AppError> {
    Ok(state.content_service.remove(id).await?)
}

#[tauri::command]
pub async fn revalidate_video(state: State<'_, AppState>, id: Uuid) -> Result<Video, AppError> {
    Ok(state.content_service.revalidate(id).await?)
}

#[tauri::command]
pub async fn regenerate_video_thumbnail(
    state: State<'_, AppState>,
    id: Uuid,
) -> Result<Video, AppError> {
    Ok(state.content_service.regenerate_thumbnail(id).await?)
}

#[tauri::command]
pub async fn reveal_video_in_file_manager(
    state: State<'_, AppState>,
    id: Uuid,
) -> Result<(), AppError> {
    let path = state.content_service.confirm_file_exists(id).await?;
    reveal_in_file_manager(std::path::Path::new(&path))?;
    Ok(())
}
