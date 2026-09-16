use serde::Deserialize;
use tauri::State;
use uuid::Uuid;

use crate::application::metadata_template_service::MetadataUpdateInput;
use crate::domain::platform::Platform;
use crate::domain::publication::Publication;
use crate::domain::publishing::{
    HashtagSet, MetadataTemplate, MetadataValidationIssue, RenderedMetadata, TemplateKind,
};
use crate::error::AppError;
use crate::state::AppState;

/// Wire shape for one optional field in [`UpdatePublicationMetadataRequest`]
/// — `set: false` (or the field omitted entirely) means "leave this field
/// exactly as it is," `set: true` with `value: None` means "clear it back
/// to automatic resolution," and `set: true` with `value: Some(x)` means
/// "set it to `x`." A plain nullable JSON field can't express these three
/// states unambiguously, which is exactly the bug this shape avoids.
#[derive(Debug, Deserialize, Default)]
pub struct MetadataFieldUpdate<T> {
    pub set: bool,
    #[serde(default)]
    pub value: Option<T>,
}

fn to_double_option<T>(field: Option<MetadataFieldUpdate<T>>) -> Option<Option<T>> {
    field.and_then(|f| if f.set { Some(f.value) } else { None })
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdatePublicationMetadataRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub hashtags: Option<Vec<String>>,
    #[serde(default)]
    pub title_override: Option<MetadataFieldUpdate<String>>,
    #[serde(default)]
    pub description_override: Option<MetadataFieldUpdate<String>>,
    #[serde(default)]
    pub hashtags_override: Option<MetadataFieldUpdate<Vec<String>>>,
    #[serde(default)]
    pub title_template_id: Option<MetadataFieldUpdate<Uuid>>,
    #[serde(default)]
    pub description_template_id: Option<MetadataFieldUpdate<Uuid>>,
    #[serde(default)]
    pub hashtag_set_id: Option<MetadataFieldUpdate<Uuid>>,
    #[serde(default)]
    pub provider_options_override: Option<MetadataFieldUpdate<serde_json::Value>>,
}

impl From<UpdatePublicationMetadataRequest> for MetadataUpdateInput {
    fn from(req: UpdatePublicationMetadataRequest) -> Self {
        MetadataUpdateInput {
            title: req.title,
            description: req.description,
            hashtags: req.hashtags,
            title_override: to_double_option(req.title_override),
            description_override: to_double_option(req.description_override),
            hashtags_override: to_double_option(req.hashtags_override),
            title_template_id: to_double_option(req.title_template_id),
            description_template_id: to_double_option(req.description_template_id),
            hashtag_set_id: to_double_option(req.hashtag_set_id),
            provider_options_override: to_double_option(req.provider_options_override),
        }
    }
}

/// Live, unsaved rendering of a publication's metadata through the full
/// precedence ladder (section 9) — what the metadata editor's preview
/// pane shows as the user changes templates/overrides.
#[tauri::command]
pub async fn preview_publication_metadata(
    state: State<'_, AppState>,
    publication_id: Uuid,
) -> Result<RenderedMetadata, AppError> {
    Ok(state
        .metadata_template_service
        .preview(publication_id)
        .await?)
}

/// The metadata actually frozen at execution time (`Publication.
/// rendered_metadata`), if this publication has ever executed —
/// `None` for one that hasn't (there's nothing frozen yet; use
/// `preview_publication_metadata` for a live view instead).
#[tauri::command]
pub async fn get_rendered_metadata(
    state: State<'_, AppState>,
    publication_id: Uuid,
) -> Result<Option<RenderedMetadata>, AppError> {
    Ok(state
        .publication_service
        .get(publication_id)
        .await?
        .and_then(|p| p.rendered_metadata))
}

/// Runs the target provider's real validation against the current live
/// resolution (section 14) — typed issues, never a raw provider message
/// the frontend has to parse.
#[tauri::command]
pub async fn validate_publication_metadata(
    state: State<'_, AppState>,
    publication_id: Uuid,
) -> Result<Vec<MetadataValidationIssue>, AppError> {
    Ok(state
        .metadata_template_service
        .validate(publication_id)
        .await?)
}

#[tauri::command]
pub async fn update_publication_metadata(
    state: State<'_, AppState>,
    publication_id: Uuid,
    request: UpdatePublicationMetadataRequest,
) -> Result<Publication, AppError> {
    Ok(state
        .metadata_template_service
        .update_publication_metadata(publication_id, request.into())
        .await?)
}

#[tauri::command]
pub async fn list_metadata_templates(
    state: State<'_, AppState>,
    workspace_id: Uuid,
) -> Result<Vec<MetadataTemplate>, AppError> {
    Ok(state
        .metadata_template_service
        .list_templates(workspace_id)
        .await?)
}

#[tauri::command]
pub async fn create_metadata_template(
    state: State<'_, AppState>,
    workspace_id: Uuid,
    channel_id: Option<Uuid>,
    platform: Option<Platform>,
    kind: TemplateKind,
    template_text: String,
) -> Result<MetadataTemplate, AppError> {
    Ok(state
        .metadata_template_service
        .create_template(workspace_id, channel_id, platform, kind, template_text)
        .await?)
}

#[tauri::command]
pub async fn update_metadata_template(
    state: State<'_, AppState>,
    id: Uuid,
    channel_id: Option<Uuid>,
    platform: Option<Platform>,
    template_text: String,
) -> Result<MetadataTemplate, AppError> {
    Ok(state
        .metadata_template_service
        .update_template(id, channel_id, platform, template_text)
        .await?)
}

#[tauri::command]
pub async fn delete_metadata_template(
    state: State<'_, AppState>,
    id: Uuid,
) -> Result<(), AppError> {
    Ok(state.metadata_template_service.delete_template(id).await?)
}

#[tauri::command]
pub async fn list_hashtag_sets(
    state: State<'_, AppState>,
    workspace_id: Uuid,
) -> Result<Vec<HashtagSet>, AppError> {
    Ok(state
        .metadata_template_service
        .list_hashtag_sets(workspace_id)
        .await?)
}

#[tauri::command]
pub async fn create_hashtag_set(
    state: State<'_, AppState>,
    workspace_id: Uuid,
    channel_id: Option<Uuid>,
    platform: Option<Platform>,
    name: String,
    hashtags: Vec<String>,
) -> Result<HashtagSet, AppError> {
    Ok(state
        .metadata_template_service
        .create_hashtag_set(workspace_id, channel_id, platform, name, hashtags)
        .await?)
}

#[tauri::command]
pub async fn update_hashtag_set(
    state: State<'_, AppState>,
    id: Uuid,
    channel_id: Option<Uuid>,
    platform: Option<Platform>,
    name: String,
    hashtags: Vec<String>,
) -> Result<HashtagSet, AppError> {
    Ok(state
        .metadata_template_service
        .update_hashtag_set(id, channel_id, platform, name, hashtags)
        .await?)
}

#[tauri::command]
pub async fn delete_hashtag_set(state: State<'_, AppState>, id: Uuid) -> Result<(), AppError> {
    Ok(state
        .metadata_template_service
        .delete_hashtag_set(id)
        .await?)
}
