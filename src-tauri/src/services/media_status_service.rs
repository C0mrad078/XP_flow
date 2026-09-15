use std::sync::Arc;

use crate::domain::ports::media_service::{MediaService, MediaToolchainStatus};

/// Cross-cutting technical service (not tied to a single domain aggregate)
/// that answers "can XP FLOW currently touch media files?" for the
/// Settings/About screen. Wraps the `MediaService` port so callers don't
/// need to know it is backed by shelling out to `ffmpeg -version`.
pub struct MediaStatusService {
    media_service: Arc<dyn MediaService>,
}

impl MediaStatusService {
    pub fn new(media_service: Arc<dyn MediaService>) -> Self {
        Self { media_service }
    }

    pub async fn status(&self) -> MediaToolchainStatus {
        self.media_service.detect().await
    }
}
