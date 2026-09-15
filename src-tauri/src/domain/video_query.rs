use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::video_status::{AvailabilityStatus, Orientation, ValidationStatus, VideoPriority};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoSort {
    NewestImported,
    OldestImported,
    Filename,
    Duration,
    FileSize,
    Channel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelFilter {
    Any(Uuid),
    Unassigned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateFilter {
    PossibleDuplicates,
}

/// Composable filter/sort/pagination input for the Content Library
/// (sections 37-40). Built once in `application::content_service` from the
/// frontend's query params and passed straight to the repository — the
/// query stays entirely database-side so the library performs at
/// tens-of-thousands-of-rows scale (section 40).
#[derive(Debug, Clone)]
pub struct VideoListQuery {
    pub workspace_id: Uuid,
    pub search: Option<String>,
    pub channel: Option<ChannelFilter>,
    pub source_id: Option<Uuid>,
    pub validation_status: Option<ValidationStatus>,
    pub availability_status: Option<AvailabilityStatus>,
    pub orientation: Option<Orientation>,
    pub priority: Option<VideoPriority>,
    pub duplicate: Option<DuplicateFilter>,
    pub include_archived: bool,
    pub sort: VideoSort,
    pub page: i64,
    pub page_size: i64,
}

impl VideoListQuery {
    pub fn new(workspace_id: Uuid) -> Self {
        Self {
            workspace_id,
            search: None,
            channel: None,
            source_id: None,
            validation_status: None,
            availability_status: None,
            orientation: None,
            priority: None,
            duplicate: None,
            include_archived: false,
            sort: VideoSort::NewestImported,
            page: 0,
            page_size: 60,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct VideoPage {
    pub items: Vec<super::video::Video>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

/// Backs the Content Library's summary bar (section 74) — a handful of
/// `COUNT(*) ... WHERE` queries, not a full row scan.
#[derive(Debug, Clone, Copy, Default, serde::Serialize)]
pub struct VideoLibrarySummary {
    pub total: i64,
    pub ready: i64,
    pub processing: i64,
    pub duplicates: i64,
    pub invalid: i64,
    pub missing: i64,
    pub archived: i64,
    pub unassigned: i64,
}
