use std::path::Path;
use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::activity_event::{ActivityCategory, ActivityLevel};
use crate::domain::duplicate_match::DuplicateMatch;
use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::ports::repositories::{DuplicateMatchRepository, VideoRepository};
use crate::domain::video::Video;
use crate::domain::video_query::{VideoLibrarySummary, VideoListQuery, VideoPage};
use crate::domain::video_status::{Orientation, VideoPriority, VideoWarning};

use super::activity_service::ActivityService;
use super::media_ingestion_service::MediaIngestionService;

/// The Content Details panel (section 41) needs more than the raw row —
/// orientation, aspect ratio and warnings are derived (never stored, see
/// `Video::orientation`/`Video::warnings`), and near-duplicate matches come
/// from a separate table.
#[derive(Debug, Clone, Serialize)]
pub struct VideoDetail {
    #[serde(flatten)]
    pub video: Video,
    pub orientation: Option<Orientation>,
    pub aspect_ratio: Option<String>,
    pub warnings: Vec<VideoWarning>,
    pub possible_duplicates: Vec<DuplicateMatch>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateVideoInput {
    pub display_title: Option<String>,
    pub channel_id: Option<Option<Uuid>>,
    pub priority: Option<VideoPriority>,
    pub notes: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
pub struct BulkUpdateInput {
    pub channel_id: Option<Option<Uuid>>,
    pub priority: Option<VideoPriority>,
    pub archived: Option<bool>,
}

/// Backs the Content Library screen (section 33-41): search/filter/sort/
/// paginate, per-video mutation, archiving, removal, and re-running
/// validation/thumbnail generation on demand. Every write that can affect
/// what's on disk (revalidate, regenerate thumbnail) delegates to
/// [`MediaIngestionService`] rather than duplicating FFmpeg-calling logic
/// (section 13/96 — one ingestion pipeline, no parallel implementations).
pub struct ContentService {
    video_repo: Arc<dyn VideoRepository>,
    duplicate_repo: Arc<dyn DuplicateMatchRepository>,
    ingestion: Arc<MediaIngestionService>,
    activity_service: Arc<ActivityService>,
}

impl ContentService {
    pub fn new(
        video_repo: Arc<dyn VideoRepository>,
        duplicate_repo: Arc<dyn DuplicateMatchRepository>,
        ingestion: Arc<MediaIngestionService>,
        activity_service: Arc<ActivityService>,
    ) -> Self {
        Self {
            video_repo,
            duplicate_repo,
            ingestion,
            activity_service,
        }
    }

    pub async fn list(&self, query: VideoListQuery) -> DomainResult<VideoPage> {
        self.video_repo.list_paginated(&query).await
    }

    pub async fn summary(&self, workspace_id: Uuid) -> DomainResult<VideoLibrarySummary> {
        self.video_repo.summary(workspace_id).await
    }

    pub async fn get_detail(&self, id: Uuid) -> DomainResult<Option<VideoDetail>> {
        let Some(video) = self.video_repo.get(id).await? else {
            return Ok(None);
        };
        let possible_duplicates = self
            .duplicate_repo
            .list_for_video(id)
            .await
            .unwrap_or_default();

        Ok(Some(VideoDetail {
            orientation: video.orientation(),
            aspect_ratio: video.aspect_ratio_label(),
            warnings: video.warnings(),
            possible_duplicates,
            video,
        }))
    }

    async fn load(&self, id: Uuid) -> DomainResult<Video> {
        self.video_repo
            .get(id)
            .await?
            .ok_or_else(|| DomainError::NotFound {
                entity: "Video",
                id: id.to_string(),
            })
    }

    /// Notes/priority/channel/title are safe for the frontend to update
    /// optimistically (section 72) — none of them can make a valid import
    /// look like it succeeded when it didn't.
    pub async fn update(&self, id: Uuid, input: UpdateVideoInput) -> DomainResult<Video> {
        let mut video = self.load(id).await?;

        if let Some(title) = input.display_title {
            let trimmed = title.trim();
            if trimmed.is_empty() {
                return Err(DomainError::Validation("title cannot be empty".into()));
            }
            video.display_title = trimmed.to_string();
        }
        if let Some(channel_id) = input.channel_id {
            video.channel_id = channel_id;
        }
        if let Some(priority) = input.priority {
            video.priority = priority;
        }
        if let Some(notes) = input.notes {
            video.notes = notes;
        }
        video.updated_at = Utc::now();

        self.video_repo.update(&video).await?;
        Ok(video)
    }

    pub async fn bulk_update(&self, ids: &[Uuid], input: BulkUpdateInput) -> DomainResult<usize> {
        let mut updated = 0;
        for id in ids {
            let Ok(mut video) = self.load(*id).await else {
                continue;
            };
            if let Some(channel_id) = input.channel_id {
                video.channel_id = channel_id;
            }
            if let Some(priority) = input.priority {
                video.priority = priority;
            }
            if let Some(archived) = input.archived {
                video.archived = archived;
            }
            video.updated_at = Utc::now();
            if self.video_repo.update(&video).await.is_ok() {
                updated += 1;
            }
        }
        Ok(updated)
    }

    pub async fn set_archived(&self, id: Uuid, archived: bool) -> DomainResult<Video> {
        let mut video = self.load(id).await?;
        video.archived = archived;
        video.updated_at = Utc::now();
        self.video_repo.update(&video).await?;
        Ok(video)
    }

    /// Removes the database record (and its generated thumbnail). The
    /// original video file on disk is never touched (section 51/65).
    pub async fn remove(&self, id: Uuid) -> DomainResult<()> {
        let video = self.load(id).await?;

        if let Some(thumbnail_path) = &video.thumbnail_path {
            let _ = tokio::fs::remove_file(thumbnail_path).await;
        }

        self.video_repo.delete(id).await?;
        self.activity_service
            .log(
                ActivityCategory::Content,
                ActivityLevel::Info,
                format!("Removed \"{}\" from XP FLOW", video.display_title),
            )
            .await?;
        Ok(())
    }

    pub async fn revalidate(&self, id: Uuid) -> DomainResult<Video> {
        let mut video = self.load(id).await?;
        self.ingestion
            .revalidate(&mut video)
            .await
            .map_err(|e| DomainError::Validation(e.user_message()))?;
        video.updated_at = Utc::now();
        self.video_repo.update(&video).await?;
        Ok(video)
    }

    pub async fn regenerate_thumbnail(&self, id: Uuid) -> DomainResult<Video> {
        let mut video = self.load(id).await?;
        let thumbnail_path = self
            .ingestion
            .regenerate_thumbnail(&video)
            .await
            .map_err(|e| DomainError::Validation(e.user_message()))?;
        video.thumbnail_path = Some(thumbnail_path.display().to_string());
        video.updated_at = Utc::now();
        self.video_repo.update(&video).await?;
        Ok(video)
    }

    /// section 45: `reveal_in_file_manager` lives in
    /// `infrastructure::filesystem`; this just confirms the file still
    /// exists before the command shells out to the OS.
    pub async fn confirm_file_exists(&self, id: Uuid) -> DomainResult<String> {
        let video = self.load(id).await?;
        if !Path::new(&video.file_path).is_file() {
            return Err(DomainError::Validation(
                "this video's file could not be found on disk".into(),
            ));
        }
        Ok(video.file_path)
    }
}
