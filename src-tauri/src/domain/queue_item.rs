use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Placement of a [`super::publication::Publication`] inside the operator's
/// manual queue.
///
/// Design decision (Phase 3 section 12): `priority` is *not* duplicated
/// here — `Publication::priority` is the single source of truth, so this
/// struct only owns what is genuinely specific to manual-queue membership:
/// its existence (a `QueueItem` row exists only while a publication is under
/// active manual-queue management — created on `Ready -> Queued`, kept
/// through `Scheduled`, removed on `Cancelled`/`Archived`) and its manual
/// ordering position. `position` is only meaningful for *unscheduled* queue
/// items (section 15) — once a publication has a `scheduled_at`, ordering is
/// derived from that timestamp instead, never from `position`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub publication_id: Uuid,
    pub position: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl QueueItem {
    pub fn new(workspace_id: Uuid, publication_id: Uuid, position: i64) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            workspace_id,
            publication_id,
            position,
            created_at: now,
            updated_at: now,
        }
    }
}
