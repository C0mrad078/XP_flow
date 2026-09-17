use tauri::Emitter;

use crate::domain::ports::progress_publisher::{ProgressPublisher, PublishProgressEvent};

/// The event name every frontend listener subscribes to (section 28) —
/// one constant, never a string literal duplicated on both sides.
pub const PUBLISH_PROGRESS_EVENT: &str = "publish-progress";

/// Broadcasts every event globally (`AppHandle::emit`, not a
/// window-scoped `emit_to`) — XP FLOW is a single-window desktop app, so
/// there's exactly one listener in practice, but a global broadcast
/// still means a listener never has to know which window/label to
/// target.
pub struct TauriProgressPublisher {
    app_handle: tauri::AppHandle,
}

impl TauriProgressPublisher {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self { app_handle }
    }
}

impl ProgressPublisher for TauriProgressPublisher {
    fn publish(&self, event: PublishProgressEvent) {
        // Best-effort: a progress tick is inherently transient (section
        // 29) — if no window is currently listening, there is nothing
        // useful to retry or fall back to.
        let _ = self.app_handle.emit(PUBLISH_PROGRESS_EVENT, event);
    }
}
