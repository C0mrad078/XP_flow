use serde::Serialize;
use uuid::Uuid;

use crate::domain::platform::Platform;

/// Live upload progress broadcast to the frontend (Phase 5.1 section 28)
/// — always real, provider-acknowledged bytes (never invented), and
/// always transient: the frontend must never treat one of these as a
/// final result, only `Publication`'s own persisted status is
/// authoritative (section 29).
#[derive(Debug, Clone, Serialize)]
pub struct PublishProgressEvent {
    pub publication_id: Uuid,
    pub attempt_id: Uuid,
    pub platform: Platform,
    pub bytes_uploaded: i64,
    pub bytes_total: Option<i64>,
    /// `None` when `bytes_total` isn't known yet — the frontend shows an
    /// indeterminate indicator rather than a fabricated percentage.
    pub percentage: Option<f64>,
    pub phase: PublishProgressPhase,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PublishProgressPhase {
    Uploading,
    Finalizing,
}

/// The one seam `PublishingEngineService` pushes live progress through
/// (section 19's "one authoritative place" discipline, applied here too)
/// — a plain trait so engine tests never need a real Tauri `AppHandle`.
pub trait ProgressPublisher: Send + Sync {
    fn publish(&self, event: PublishProgressEvent);
}

/// Test/no-op implementation — used by every engine test and anywhere
/// else a real event bus isn't available.
pub struct NullProgressPublisher;

impl ProgressPublisher for NullProgressPublisher {
    fn publish(&self, _event: PublishProgressEvent) {}
}
