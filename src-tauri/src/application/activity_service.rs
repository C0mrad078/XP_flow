use std::sync::Arc;

use crate::domain::activity_event::{ActivityCategory, ActivityEvent, ActivityLevel};
use crate::domain::errors::DomainResult;
use crate::domain::ports::repositories::ActivityRepository;

/// Thin façade over `ActivityRepository`. Every subsystem that wants to
/// leave a trace in the Activity screen (section 30) goes through
/// `log(...)` rather than writing to the repository directly, so call
/// sites read as intent ("log a warning") instead of persistence detail.
pub struct ActivityService {
    repo: Arc<dyn ActivityRepository>,
}

impl ActivityService {
    pub fn new(repo: Arc<dyn ActivityRepository>) -> Self {
        Self { repo }
    }

    pub async fn log(
        &self,
        category: ActivityCategory,
        level: ActivityLevel,
        message: impl Into<String>,
    ) -> DomainResult<ActivityEvent> {
        let event = ActivityEvent::new(category, level, message);
        self.repo.record(&event).await?;
        Ok(event)
    }

    pub async fn list_recent(&self, limit: i64) -> DomainResult<Vec<ActivityEvent>> {
        self.repo.list_recent(limit).await
    }
}
