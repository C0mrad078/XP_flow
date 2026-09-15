use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Where a [`super::video::Video`] originally came from. Kept separate from
/// `Video` so a future Cut.pro folder watcher (or a manual drag-and-drop
/// import, or a future remote sync) can each record their own provenance
/// without overloading the video row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoSourceType {
    LocalFile,
    CutProExport,
    ManualUpload,
}

impl VideoSourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            VideoSourceType::LocalFile => "local_file",
            VideoSourceType::CutProExport => "cut_pro_export",
            VideoSourceType::ManualUpload => "manual_upload",
        }
    }
}

impl std::str::FromStr for VideoSourceType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "local_file" => Ok(VideoSourceType::LocalFile),
            "cut_pro_export" => Ok(VideoSourceType::CutProExport),
            "manual_upload" => Ok(VideoSourceType::ManualUpload),
            other => Err(format!("unknown video source type: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoSource {
    pub id: Uuid,
    pub video_id: Uuid,
    pub source_type: VideoSourceType,
    pub origin_path: String,
    pub metadata_json: Option<String>,
    pub created_at: DateTime<Utc>,
}
