use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A `Video` is the source media asset imported into XP FLOW. It is
/// deliberately separate from [`super::publication::Publication`]: one video
/// can produce many publications (one per platform, or repeated posts).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Video {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub title: String,
    pub duration_seconds: Option<i64>,
    pub file_path: String,
    pub file_size_bytes: Option<i64>,
    pub checksum: Option<String>,
    pub imported_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Video {
    pub fn new(workspace_id: Uuid, title: impl Into<String>, file_path: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            workspace_id,
            title: title.into(),
            duration_seconds: None,
            file_path: file_path.into(),
            file_size_bytes: None,
            checksum: None,
            imported_at: now,
            created_at: now,
            updated_at: now,
        }
    }
}
