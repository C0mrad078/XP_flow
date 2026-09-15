use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::activity_event::{ActivityCategory, ActivityLevel};
use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::ports::repositories::{VideoRepository, VideoSourceRepository};
use crate::domain::video_source::{VideoSource, VideoSourceType};

use super::activity_service::ActivityService;

#[derive(Debug, Clone, Serialize)]
pub struct SourceSummary {
    #[serde(flatten)]
    pub source: VideoSource,
    pub files_indexed: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateSourceInput {
    pub name: String,
    pub source_type: VideoSourceType,
    pub folder_path: String,
    pub channel_id: Option<Uuid>,
    pub recursive: bool,
    pub watch_enabled: bool,
}

/// `channel_id` uses `serde_with`'s `double_option` — see the identical
/// note on `content_service::UpdateVideoInput` — so an "Unassign channel"
/// action in the UI (JSON `null`) is distinguishable from simply not
/// touching the field.
#[derive(Debug, Deserialize)]
pub struct UpdateSourceInput {
    pub name: Option<String>,
    #[serde(default, with = "::serde_with::rust::double_option")]
    pub channel_id: Option<Option<Uuid>>,
    pub enabled: Option<bool>,
    pub recursive: Option<bool>,
    pub watch_enabled: Option<bool>,
}

/// Orchestrates the "Folder Sources" screen (section 52) and the
/// Cut.pro/watch-folder "Add Content Folder" wizard (section 53/54).
/// `VideoSourceRepository` is the persistence port; actually starting or
/// stopping an OS-level watcher for a source lives in
/// `services::job_runner::JobRunner`, which owns the runtime
/// `FolderWatcherService` — this service only manages the source's
/// *configuration*.
pub struct SourceService {
    source_repo: Arc<dyn VideoSourceRepository>,
    video_repo: Arc<dyn VideoRepository>,
    activity_service: Arc<ActivityService>,
}

impl SourceService {
    pub fn new(
        source_repo: Arc<dyn VideoSourceRepository>,
        video_repo: Arc<dyn VideoRepository>,
        activity_service: Arc<ActivityService>,
    ) -> Self {
        Self {
            source_repo,
            video_repo,
            activity_service,
        }
    }

    /// Every workspace gets exactly one implicit "Manual Import" source
    /// (used by the file dialog and drag-and-drop) — created lazily the
    /// first time it's needed rather than special-cased at the database
    /// level.
    pub async fn ensure_manual_import_source(
        &self,
        workspace_id: Uuid,
    ) -> DomainResult<VideoSource> {
        let sources = self.source_repo.list_for_workspace(workspace_id).await?;
        if let Some(existing) = sources
            .into_iter()
            .find(|s| s.source_type == VideoSourceType::ManualImport)
        {
            return Ok(existing);
        }

        let source = VideoSource::manual_import(workspace_id);
        self.source_repo.create(&source).await?;
        Ok(source)
    }

    pub async fn list_with_summary(&self, workspace_id: Uuid) -> DomainResult<Vec<SourceSummary>> {
        let sources = self.source_repo.list_for_workspace(workspace_id).await?;
        let mut summaries = Vec::with_capacity(sources.len());
        for source in sources {
            let files_indexed = self
                .video_repo
                .count_for_source(source.id)
                .await
                .unwrap_or(0);
            summaries.push(SourceSummary {
                source,
                files_indexed,
            });
        }
        Ok(summaries)
    }

    pub async fn get(&self, id: Uuid) -> DomainResult<Option<VideoSource>> {
        self.source_repo.get(id).await
    }

    pub async fn create_folder_source(
        &self,
        workspace_id: Uuid,
        input: CreateSourceInput,
    ) -> DomainResult<VideoSource> {
        let trimmed_path = input.folder_path.trim();
        if trimmed_path.is_empty() {
            return Err(DomainError::Validation("a folder path is required".into()));
        }
        if !std::path::Path::new(trimmed_path).is_dir() {
            return Err(DomainError::Validation(
                "that folder could not be found".into(),
            ));
        }
        if matches!(input.source_type, VideoSourceType::ManualImport) {
            return Err(DomainError::Validation(
                "Manual Import is a built-in source and cannot be created again".into(),
            ));
        }

        let source = VideoSource::new_folder(
            workspace_id,
            input.name.trim(),
            input.source_type,
            trimmed_path,
            input.channel_id,
            input.recursive,
            input.watch_enabled,
        );
        self.source_repo.create(&source).await?;

        self.activity_service
            .log(
                ActivityCategory::System,
                ActivityLevel::Success,
                format!("Content source \"{}\" added ({trimmed_path})", source.name),
            )
            .await?;

        Ok(source)
    }

    pub async fn update_source(
        &self,
        id: Uuid,
        input: UpdateSourceInput,
    ) -> DomainResult<VideoSource> {
        let mut source = self
            .source_repo
            .get(id)
            .await?
            .ok_or_else(|| DomainError::NotFound {
                entity: "VideoSource",
                id: id.to_string(),
            })?;

        if let Some(name) = input.name {
            source.name = name;
        }
        if let Some(channel_id) = input.channel_id {
            source.channel_id = channel_id;
        }
        if let Some(enabled) = input.enabled {
            source.enabled = enabled;
        }
        if let Some(recursive) = input.recursive {
            source.recursive = recursive;
        }
        if let Some(watch_enabled) = input.watch_enabled {
            source.watch_enabled = watch_enabled;
        }
        source.updated_at = Utc::now();

        self.source_repo.update(&source).await?;
        Ok(source)
    }

    pub async fn delete_source(&self, id: Uuid) -> DomainResult<()> {
        let source = self.source_repo.get(id).await?;
        if let Some(source) = &source {
            if matches!(source.source_type, VideoSourceType::ManualImport) {
                return Err(DomainError::Validation(
                    "the Manual Import source cannot be removed".into(),
                ));
            }
        }
        self.source_repo.delete(id).await
    }
}
