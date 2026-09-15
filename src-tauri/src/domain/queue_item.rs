use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Placement of a [`super::publication::Publication`] inside the operator
/// queue. Kept as its own entity (rather than columns on `Publication`) so
/// future view modes (timeline, kanban, calendar) can reorder/re-prioritize
/// without touching the publication record itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: Uuid,
    pub publication_id: Uuid,
    pub priority: i32,
    pub position: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl QueueItem {
    pub fn new(publication_id: Uuid, position: i32) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            publication_id,
            priority: 0,
            position,
            created_at: now,
            updated_at: now,
        }
    }
}
