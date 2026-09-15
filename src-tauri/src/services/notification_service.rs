use std::sync::Arc;

use uuid::Uuid;

use crate::domain::errors::DomainResult;
use crate::domain::notification::{Notification, NotificationType};
use crate::domain::ports::repositories::NotificationRepository;

/// Backs the Notification Center (section 40). Kept separate from
/// `ActivityService`: activity is a full audit trail, notifications are the
/// smaller, dismissible/read-tracked subset surfaced in the shell's bell
/// icon.
pub struct NotificationService {
    repo: Arc<dyn NotificationRepository>,
}

impl NotificationService {
    pub fn new(repo: Arc<dyn NotificationRepository>) -> Self {
        Self { repo }
    }

    pub async fn notify(
        &self,
        notification_type: NotificationType,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> DomainResult<Notification> {
        let notification = Notification::new(notification_type, title, message);
        self.repo.create(&notification).await?;
        Ok(notification)
    }

    pub async fn list_recent(&self, limit: i64) -> DomainResult<Vec<Notification>> {
        self.repo.list_recent(limit).await
    }

    pub async fn unread_count(&self) -> DomainResult<i64> {
        self.repo.unread_count().await
    }

    pub async fn mark_read(&self, id: Uuid) -> DomainResult<()> {
        self.repo.mark_read(id).await
    }
}
