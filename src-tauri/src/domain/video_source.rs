use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// What kind of [`VideoSource`] this is. `ManualImport` is the implicit
/// default source every workspace gets (used by the "Import Videos"
/// dialog and drag-and-drop, section 12); `CutproFolder` and
/// `WatchFolder` back a monitored directory (sections 8/54) — the only
/// difference between them is which defaults the "Add Content Folder"
/// wizard pre-fills (section 54).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoSourceType {
    CutproFolder,
    ManualImport,
    WatchFolder,
}

impl VideoSourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            VideoSourceType::CutproFolder => "cutpro_folder",
            VideoSourceType::ManualImport => "manual_import",
            VideoSourceType::WatchFolder => "watch_folder",
        }
    }
}

impl std::str::FromStr for VideoSourceType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "cutpro_folder" => Ok(VideoSourceType::CutproFolder),
            "manual_import" => Ok(VideoSourceType::ManualImport),
            "watch_folder" => Ok(VideoSourceType::WatchFolder),
            other => Err(format!("unknown video source type: {other}")),
        }
    }
}

/// A configured place XP FLOW ingests video from: a watched Cut.pro export
/// folder, an ad-hoc watched folder, or the implicit "Manual Import"
/// source every workspace is created with (used by the file-picker and
/// drag-and-drop import paths, which don't need their own folder).
///
/// One `VideoSource` can optionally map to one [`super::channel::Channel`]
/// (section 9) — every video ingested through it defaults to that channel
/// unless the user overrides it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoSource {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub source_type: VideoSourceType,
    pub folder_path: Option<String>,
    pub channel_id: Option<Uuid>,
    pub enabled: bool,
    pub recursive: bool,
    pub watch_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_scan_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

impl VideoSource {
    pub fn manual_import(workspace_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            workspace_id,
            name: "Manual Import".to_string(),
            source_type: VideoSourceType::ManualImport,
            folder_path: None,
            channel_id: None,
            enabled: true,
            recursive: false,
            watch_enabled: false,
            created_at: now,
            updated_at: now,
            last_scan_at: None,
            last_error: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_folder(
        workspace_id: Uuid,
        name: impl Into<String>,
        source_type: VideoSourceType,
        folder_path: impl Into<String>,
        channel_id: Option<Uuid>,
        recursive: bool,
        watch_enabled: bool,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            workspace_id,
            name: name.into(),
            source_type,
            folder_path: Some(folder_path.into()),
            channel_id,
            enabled: true,
            recursive,
            watch_enabled,
            created_at: now,
            updated_at: now,
            last_scan_at: None,
            last_error: None,
        }
    }

    /// Whether this source is a folder that can be scanned/watched at all
    /// (the Manual Import source has no folder and is never watched).
    pub fn is_folder_backed(&self) -> bool {
        self.folder_path.is_some()
    }
}
