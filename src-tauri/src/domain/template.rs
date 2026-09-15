use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::platform::Platform;

/// A reusable metadata template (title/description/hashtags) that can be
/// applied to a publication when it is created. `platform` is optional so a
/// template can either be platform-specific or generic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub title_template: Option<String>,
    pub description_template: Option<String>,
    pub hashtags: Vec<String>,
    pub platform: Option<Platform>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
