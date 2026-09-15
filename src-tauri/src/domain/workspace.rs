use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A `Workspace` is the top-level container for everything a user manages in
/// XP FLOW: channels, videos, publications, templates and settings all live
/// inside exactly one workspace. Phase 1 assumes a single local workspace,
/// but the schema and domain model already scope every child entity by
/// `workspace_id` so multi-workspace support does not require a redesign.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Workspace {
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            created_at: now,
            updated_at: now,
        }
    }
}
