use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A `Channel` groups a niche/brand of content together across platforms.
/// One channel can eventually hold one `PlatformAccount` per platform
/// (YouTube, TikTok, Kwai).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub niche: Option<String>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Channel {
    pub fn new(workspace_id: Uuid, name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            workspace_id,
            name: name.into(),
            niche: None,
            description: None,
            created_at: now,
            updated_at: now,
        }
    }
}
